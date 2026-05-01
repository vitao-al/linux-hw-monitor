/// Stress Test module – Selective Hardware Stress (Linux)
///
/// Implements CPU / RAM / GPU / Disk stress methods via external tools
/// (stress-ng, fio, glmark2) with:
///   • nice -n 19  (lowest CPU priority)
///   • ionice -c 3 (idle I/O class for disk tests)
///   • OOM guard   (min 512 MB free RAM before RAM tests)
///   • Safety watchdog: SIGKILL when temp > user threshold
///   • Session report: peak temp, throttling flag, RAPL energy
///
/// Thread-safety note: GTK widgets are not Send. All UI updates happen on the
/// main thread via the 1-second glib timer. The watchdog thread only writes to
/// Arc<Mutex<…>> shared state; the UI timer reads and reacts.
use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use adw::prelude::*;
use gtk4 as gtk;
use glib;
use crate::i18n::t;

// ────────────────────────────────────────────────────────────────────────────
// Public types
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StressComponent {
    Cpu,
    Ram,
    Gpu,
    Disk,
}

impl fmt::Display for StressComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StressComponent::Cpu => write!(f, "CPU"),
            StressComponent::Ram => write!(f, "RAM"),
            StressComponent::Gpu => write!(f, "GPU"),
            StressComponent::Disk => write!(f, "Disk"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StressMethod {
    // CPU
    CpuMatrix,
    CpuPi,
    CpuAvx,
    // RAM
    RamPattern,
    RamVm,
    // GPU
    GpuGlmark2,
    GpuTessellation,
    // Disk
    DiskSeq,
    DiskRandom,
}

impl StressMethod {
    pub fn label(self) -> String {
        match self {
            StressMethod::CpuMatrix => t("Matrix Product"),
            StressMethod::CpuPi => t("Pi Calculation"),
            StressMethod::CpuAvx => t("AVX / FPU"),
            StressMethod::RamPattern => t("Pattern Write"),
            StressMethod::RamVm => t("Stress-NG VM"),
            StressMethod::GpuGlmark2 => t("GLmark2 Nodes"),
            StressMethod::GpuTessellation => t("Tessellation"),
            StressMethod::DiskSeq => t("Sequential R/W"),
            StressMethod::DiskRandom => t("Random IOPS"),
        }
    }

    pub fn intensity_label(self) -> String {
        match self {
            StressMethod::CpuMatrix => t("Low / Med"),
            StressMethod::CpuPi => t("Medium"),
            StressMethod::CpuAvx => t("Extreme"),
            StressMethod::RamPattern => t("Low"),
            StressMethod::RamVm => t("High"),
            StressMethod::GpuGlmark2 => t("Medium"),
            StressMethod::GpuTessellation => t("High"),
            StressMethod::DiskSeq => t("Low"),
            StressMethod::DiskRandom => t("High"),
        }
    }

    /// CSS class applied to the badge label.
    pub fn badge_css_class(self) -> &'static str {
        match self {
            StressMethod::CpuMatrix => "badge-med",
            StressMethod::CpuPi => "badge-med",
            StressMethod::CpuAvx => "badge-extreme",
            StressMethod::RamPattern => "badge-low",
            StressMethod::RamVm => "badge-high",
            StressMethod::GpuGlmark2 => "badge-med",
            StressMethod::GpuTessellation => "badge-high",
            StressMethod::DiskSeq => "badge-low",
            StressMethod::DiskRandom => "badge-high",
        }
    }
    pub fn component(self) -> StressComponent {
        match self {
            StressMethod::CpuMatrix | StressMethod::CpuPi | StressMethod::CpuAvx => {
                StressComponent::Cpu
            }
            StressMethod::RamPattern | StressMethod::RamVm => StressComponent::Ram,
            StressMethod::GpuGlmark2 | StressMethod::GpuTessellation => StressComponent::Gpu,
            StressMethod::DiskSeq | StressMethod::DiskRandom => StressComponent::Disk,
        }
    }
}

/// Snapshot produced at the end (or on stop) of a stress session.
#[derive(Debug, Clone, Default)]
pub struct StressReport {
    pub method: Option<StressMethod>,
    pub duration_secs: u64,
    pub peak_cpu_temp_c: f64,
    pub peak_gpu_temp_c: f64,
    pub peak_cpu_freq_mhz: f64,
    pub avg_cpu_freq_mhz: Option<f64>,
    pub thermal_throttling_detected: bool,
    pub rapl_energy_uj_start: Option<u64>,
    pub rapl_energy_uj_end: Option<u64>,
    pub start_mem_available_mb: Option<u64>,
    pub end_mem_available_mb: Option<u64>,
    pub min_mem_available_mb: Option<u64>,
    pub load_avg_1m: Option<f64>,
    pub stopped_by_watchdog: bool,
    pub stopped_manually: bool,
    pub exit_code: Option<i32>,
    pub error: Option<String>,
}

impl StressReport {
    pub fn energy_joules(&self) -> Option<f64> {
        let start = self.rapl_energy_uj_start?;
        let end = self.rapl_energy_uj_end?;
        Some((end.saturating_sub(start)) as f64 / 1_000_000.0)
    }

    pub fn average_power_w(&self) -> Option<f64> {
        let energy = self.energy_joules()?;
        if self.duration_secs == 0 {
            return None;
        }
        Some(energy / self.duration_secs as f64)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Internal runner state (shared across UI thread and worker thread)
// ────────────────────────────────────────────────────────────────────────────

struct RunnerState {
    child: Option<Child>,
    method: Option<StressMethod>,
    start: Option<Instant>,
    peak_cpu_temp: f64,
    peak_gpu_temp: f64,
    peak_cpu_freq_mhz: f64,
    cpu_freq_sum_mhz: f64,
    cpu_freq_samples: u64,
    prev_cpu_freq_mhz: f64,
    throttle_samples_below: u32,
    rapl_start: Option<u64>,
    start_mem_available_kb: Option<u64>,
    end_mem_available_kb: Option<u64>,
    min_mem_available_kb: Option<u64>,
    load_avg_1m: Option<f64>,
    stopped_by_watchdog: bool,
    stopped_manually: bool,
    exit_code: Option<i32>,
    last_report: Option<StressReport>,
}

impl RunnerState {
    fn new() -> Self {
        RunnerState {
            child: None,
            method: None,
            start: None,
            peak_cpu_temp: 0.0,
            peak_gpu_temp: 0.0,
            peak_cpu_freq_mhz: 0.0,
            cpu_freq_sum_mhz: 0.0,
            cpu_freq_samples: 0,
            prev_cpu_freq_mhz: 0.0,
            throttle_samples_below: 0,
            rapl_start: None,
            start_mem_available_kb: None,
            end_mem_available_kb: None,
            min_mem_available_kb: None,
            load_avg_1m: None,
            stopped_by_watchdog: false,
            stopped_manually: false,
            exit_code: None,
            last_report: None,
        }
    }
}

pub struct StressRunner {
    state: Arc<Mutex<RunnerState>>,
    running: Arc<AtomicBool>,
}

impl StressRunner {
    pub fn new() -> Self {
        StressRunner {
            state: Arc::new(Mutex::new(RunnerState::new())),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Start a stress session. Returns an error string if the tool is missing or
    /// RAM guard is triggered.
    ///
    /// When the test finishes (naturally, by watchdog, or by stop()), the report
    /// is written into `last_report`. The UI timer calls `take_pending_report()`
    /// to detect completion without requiring Send closures.
    pub fn start(
        &self,
        method: StressMethod,
        temp_limit_c: f64,
    ) -> Result<(), String> {
        if self.is_running() {
            return Err(t("A stress test is already running."));
        }

        // RAM guard – ensure enough free memory before VM tests.
        if matches!(method, StressMethod::RamPattern | StressMethod::RamVm) {
            let free_kb = read_meminfo_field("MemAvailable").unwrap_or(0);
            let free_mb = free_kb / 1024;
            if free_mb < 512 {
                return Err(format!(
                    "{} ({free_mb} MB)",
                    t("Insufficient free RAM. Need at least 512 MB to safely run a memory stress test.")
                ));
            }
        }

        let child = spawn_stress_process(method)?;

        let rapl_start = read_rapl_energy_uj();
        let start_mem = read_meminfo_field("MemAvailable");

        {
            let mut st = self.state.lock().unwrap();
            st.child = Some(child);
            st.method = Some(method);
            st.start = Some(Instant::now());
            st.peak_cpu_temp = 0.0;
            st.peak_gpu_temp = 0.0;
            st.peak_cpu_freq_mhz = 0.0;
            st.cpu_freq_sum_mhz = 0.0;
            st.cpu_freq_samples = 0;
            st.prev_cpu_freq_mhz = 0.0;
            st.throttle_samples_below = 0;
            st.rapl_start = rapl_start;
            st.start_mem_available_kb = start_mem;
            st.end_mem_available_kb = None;
            st.min_mem_available_kb = start_mem;
            st.load_avg_1m = None;
            st.stopped_by_watchdog = false;
            st.stopped_manually = false;
            st.exit_code = None;
            st.last_report = None;
        }
        self.running.store(true, Ordering::SeqCst);

        // Watchdog thread: monitors temperature and kills child if limit exceeded.
        // Communicates back ONLY via the Arc<Mutex<RunnerState>> shared state;
        // no GTK widget references cross the thread boundary.
        let state_arc = Arc::clone(&self.state);
        let running_arc = Arc::clone(&self.running);
        thread::spawn(move || {
            loop {
                if !running_arc.load(Ordering::SeqCst) {
                    break;
                }

                thread::sleep(Duration::from_secs(1));

                let cpu_temp = read_cpu_temp_c().unwrap_or(0.0);
                let gpu_temp = read_gpu_temp_c().unwrap_or(0.0);
                let cpu_freq = read_cpu_freq_mhz().unwrap_or(0.0);

                let mut st = state_arc.lock().unwrap();
                if cpu_temp > st.peak_cpu_temp {
                    st.peak_cpu_temp = cpu_temp;
                }
                if gpu_temp > st.peak_gpu_temp {
                    st.peak_gpu_temp = gpu_temp;
                }
                if cpu_freq > 0.0 {
                    st.cpu_freq_sum_mhz += cpu_freq;
                    st.cpu_freq_samples += 1;
                    if cpu_freq > st.peak_cpu_freq_mhz {
                        st.peak_cpu_freq_mhz = cpu_freq;
                    }
                }
                let mem_available = read_meminfo_field("MemAvailable");
                if let Some(mem_kb) = mem_available {
                    st.min_mem_available_kb = Some(
                        st.min_mem_available_kb
                            .map(|v| v.min(mem_kb))
                            .unwrap_or(mem_kb),
                    );
                }
                st.load_avg_1m = read_loadavg_1m();

                // Throttling detection: freq drops >10% from the initial reading.
                if st.prev_cpu_freq_mhz > 0.0 && cpu_freq > 0.0 {
                    let drop_ratio = 1.0 - cpu_freq / st.prev_cpu_freq_mhz;
                    if drop_ratio > 0.10 {
                        st.throttle_samples_below += 1;
                    } else {
                        if st.throttle_samples_below > 0 {
                            st.throttle_samples_below -= 1;
                        }
                    }
                } else if cpu_freq > 0.0 {
                    st.prev_cpu_freq_mhz = cpu_freq;
                }

                let over_limit = cpu_temp > temp_limit_c || gpu_temp > temp_limit_c;
                if over_limit {
                    if let Some(child) = st.child.as_mut() {
                        let _ = child.kill();
                        st.exit_code = child.wait().ok().and_then(|s| s.code());
                    }
                    st.child = None;
                    st.stopped_by_watchdog = true;
                    st.end_mem_available_kb = read_meminfo_field("MemAvailable");
                    let report = build_report(&st);
                    st.last_report = Some(report);
                    running_arc.store(false, Ordering::SeqCst);
                    return;
                }

                // Check if child finished naturally.
                if let Some(child) = st.child.as_mut() {
                    if let Ok(Some(status)) = child.try_wait() {
                        st.exit_code = status.code();
                        st.child = None;
                        st.end_mem_available_kb = read_meminfo_field("MemAvailable");
                        let report = build_report(&st);
                        st.last_report = Some(report);
                        running_arc.store(false, Ordering::SeqCst);
                        return;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the running stress test and return the report.
    pub fn stop(&self) -> Option<StressReport> {
        if !self.is_running() {
            return None;
        }
        self.running.store(false, Ordering::SeqCst);
        let mut st = self.state.lock().unwrap();
        st.stopped_manually = true;
        if let Some(mut child) = st.child.take() {
            let _ = child.kill();
            st.exit_code = child.wait().ok().and_then(|s| s.code());
        }
        st.end_mem_available_kb = read_meminfo_field("MemAvailable");
        let report = build_report(&st);
        st.last_report = Some(report.clone());
        Some(report)
    }

    /// Current live temperatures (for the UI tick).
    pub fn live_temps(&self) -> (f64, f64) {
        (
            read_cpu_temp_c().unwrap_or(0.0),
            read_gpu_temp_c().unwrap_or(0.0),
        )
    }

    pub fn last_report(&self) -> Option<StressReport> {
        self.state.lock().unwrap().last_report.clone()
    }

    /// Take the pending report (clears it). Returns Some only if the test just
    /// finished since the last call. Used by the UI timer to detect completion.
    pub fn take_pending_report(&self) -> Option<StressReport> {
        self.state.lock().unwrap().last_report.take()
    }

    /// Elapsed seconds since the test started.
    pub fn elapsed_secs(&self) -> u64 {
        self.state
            .lock()
            .unwrap()
            .start
            .map(|s| s.elapsed().as_secs())
            .unwrap_or(0)
    }
}

fn build_report(st: &RunnerState) -> StressReport {
    let duration_secs = st
        .start
        .map(|s| s.elapsed().as_secs())
        .unwrap_or(0);
    let rapl_end = read_rapl_energy_uj();
    let avg_cpu_freq_mhz = if st.cpu_freq_samples > 0 {
        Some(st.cpu_freq_sum_mhz / st.cpu_freq_samples as f64)
    } else {
        None
    };
    StressReport {
        method: st.method,
        duration_secs,
        peak_cpu_temp_c: st.peak_cpu_temp,
        peak_gpu_temp_c: st.peak_gpu_temp,
        peak_cpu_freq_mhz: st.peak_cpu_freq_mhz,
        avg_cpu_freq_mhz,
        thermal_throttling_detected: st.throttle_samples_below >= 3,
        rapl_energy_uj_start: st.rapl_start,
        rapl_energy_uj_end: rapl_end,
        start_mem_available_mb: st.start_mem_available_kb.map(|k| k / 1024),
        end_mem_available_mb: st.end_mem_available_kb.map(|k| k / 1024),
        min_mem_available_mb: st.min_mem_available_kb.map(|k| k / 1024),
        load_avg_1m: st.load_avg_1m,
        stopped_by_watchdog: st.stopped_by_watchdog,
        stopped_manually: st.stopped_manually,
        exit_code: st.exit_code,
        error: None,
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Process spawner
// ────────────────────────────────────────────────────────────────────────────

fn spawn_stress_process(method: StressMethod) -> Result<Child, String> {
    let nproc = available_cpus();

    match method {
        StressMethod::CpuMatrix => nice_cmd(&[
            "stress-ng",
            "--cpu",
            &nproc.to_string(),
            "--cpu-method",
            "matrixprod",
        ]),
        StressMethod::CpuPi => nice_cmd(&[
            "stress-ng",
            "--cpu",
            &nproc.to_string(),
            "--cpu-method",
            "pi",
        ]),
        StressMethod::CpuAvx => nice_cmd(&[
            "stress-ng",
            "--cpu",
            &nproc.to_string(),
            "--cpu-method",
            "fpu",
        ]),
        StressMethod::RamPattern => nice_cmd(&[
            "stress-ng",
            "--vm",
            "1",
            "--vm-bytes",
            "50%",
            "--vm-method",
            "write64",
        ]),
        StressMethod::RamVm => nice_cmd(&[
            "stress-ng",
            "--vm",
            "2",
            "--vm-bytes",
            "75%",
        ]),
        StressMethod::GpuGlmark2 => {
            // glmark2 does not accept nice easily via args; wrap in sh.
            check_tool("glmark2")?;
            Command::new("nice")
                .args([
                    "-n",
                    "19",
                    "glmark2",
                    "--run-forever",
                    "--benchmark",
                    "build:use-vbo=true",
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("Failed to start glmark2: {e}"))
        }
        StressMethod::GpuTessellation => {
            check_tool("glmark2")?;
            Command::new("nice")
                .args([
                    "-n",
                    "19",
                    "glmark2",
                    "--run-forever",
                    "--benchmark",
                    "terrain:tessellation=true",
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("Failed to start glmark2 tessellation: {e}"))
        }
        StressMethod::DiskSeq => ionice_nice_cmd(&[
            "fio",
            "--name=lhm-seq",
            "--rw=readwrite",
            "--bs=4M",
            "--size=256M",
            "--runtime=86400",
            "--time_based",
            "--direct=1",
            "--unlink=1",
            "--filename=/tmp/lhm_stress_seq.tmp",
            "--output-format=terse",
        ]),
        StressMethod::DiskRandom => ionice_nice_cmd(&[
            "fio",
            "--name=lhm-rand",
            "--rw=randrw",
            "--bs=4k",
            "--size=64M",
            "--runtime=86400",
            "--time_based",
            "--direct=1",
            "--unlink=1",
            "--filename=/tmp/lhm_stress_rand.tmp",
            "--output-format=terse",
        ]),
    }
}

/// Wrap a command in `nice -n 19 <cmd> <args...>` and verify the tool exists.
fn nice_cmd(args: &[&str]) -> Result<Child, String> {
    let (tool, rest) = args.split_first().ok_or("empty args")?;
    check_tool(tool)?;
    let mut cmd = Command::new("nice");
    cmd.arg("-n").arg("19").arg(tool);
    cmd.args(rest);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to launch {tool}: {e}"))
}

/// Wrap in `ionice -c 3 nice -n 19 <tool> <args...>` (idle I/O + lowest CPU).
fn ionice_nice_cmd(args: &[&str]) -> Result<Child, String> {
    let (tool, rest) = args.split_first().ok_or("empty args")?;
    check_tool(tool)?;
    let mut cmd = Command::new("ionice");
    cmd.args(["-c", "3", "nice", "-n", "19", tool]);
    cmd.args(rest);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to launch {tool}: {e}"))
}

fn check_tool(tool: &str) -> Result<(), String> {
    Command::new("which")
        .arg(tool)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
        .filter(|s| s.success())
        .map(|_| ())
        .ok_or_else(|| format!("{}: '{tool}'", t("Required tool is not installed. Please install it via your package manager.")))
}

// ────────────────────────────────────────────────────────────────────────────
// System helpers
// ────────────────────────────────────────────────────────────────────────────

fn available_cpus() -> usize {
    thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

fn read_meminfo_field(field: &str) -> Option<u64> {
    let f = fs::File::open("/proc/meminfo").ok()?;
    for line in BufReader::new(f).lines().flatten() {
        if line.starts_with(field) {
            let kb = line.split_whitespace().nth(1)?.parse::<u64>().ok()?;
            return Some(kb);
        }
    }
    None
}

/// Read CPU temp from the first hwmon coretemp or k10temp sensor found.
fn read_cpu_temp_c() -> Option<f64> {
    read_hwmon_temp(&["coretemp", "k10temp", "zenpower", "acpitz"])
}

/// Read GPU temp from hwmon (amdgpu / nouveau / nvidia – via hwmon).
fn read_gpu_temp_c() -> Option<f64> {
    read_hwmon_temp(&["amdgpu", "nouveau", "nvidia"])
}

fn read_hwmon_temp(driver_names: &[&str]) -> Option<f64> {
    let hwmon_dir = std::path::Path::new("/sys/class/hwmon");
    let entries = fs::read_dir(hwmon_dir).ok()?;
    for entry in entries.flatten() {
        let name_path = entry.path().join("name");
        let name = fs::read_to_string(&name_path).ok()?;
        let name = name.trim();
        if !driver_names.contains(&name) {
            continue;
        }
        // Try temp1_input first, then scan others.
        for i in 1..=8u8 {
            let temp_path = entry.path().join(format!("temp{i}_input"));
            if let Ok(raw) = fs::read_to_string(&temp_path) {
                if let Ok(milli) = raw.trim().parse::<i64>() {
                    return Some(milli as f64 / 1000.0);
                }
            }
        }
    }
    None
}

/// Read current CPU frequency in MHz from /sys (average across cores).
fn read_cpu_freq_mhz() -> Option<f64> {
    let cpu_dir = std::path::Path::new("/sys/devices/system/cpu");
    let mut sum = 0.0f64;
    let mut count = 0u32;
    for i in 0..64u32 {
        let path = cpu_dir.join(format!("cpu{i}/cpufreq/scaling_cur_freq"));
        if let Ok(raw) = fs::read_to_string(&path) {
            if let Ok(khz) = raw.trim().parse::<u64>() {
                sum += khz as f64 / 1000.0;
                count += 1;
            }
        }
    }
    if count > 0 { Some(sum / count as f64) } else { None }
}

fn read_loadavg_1m() -> Option<f64> {
    let raw = fs::read_to_string("/proc/loadavg").ok()?;
    raw.split_whitespace().next()?.parse::<f64>().ok()
}

/// Read RAPL energy counter (package-0). Returns micro-joules.
fn read_rapl_energy_uj() -> Option<u64> {
    let path = "/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj";
    fs::read_to_string(path)
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
}

// ────────────────────────────────────────────────────────────────────────────
// UI – build_stress_page()
// ────────────────────────────────────────────────────────────────────────────

/// Build the "Stress Test" ViewStack page.
/// Returns a gtk::Box that can be added directly to the ViewStack.
pub fn build_stress_page() -> gtk::Box {
    use std::rc::Rc;

    let runner = Rc::new(StressRunner::new());

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);

    // ── Scrollable body ───────────────────────────────────────────────────
    let body = gtk::Box::new(gtk::Orientation::Vertical, 16);
    body.set_margin_top(16);
    body.set_margin_bottom(16);
    body.set_margin_start(16);
    body.set_margin_end(16);

    let scroll = gtk::ScrolledWindow::builder()
        .child(&body)
        .vexpand(true)
        .hexpand(true)
        .build();

    root.append(&scroll);

    // ── Method selector ───────────────────────────────────────────────────
    let method_group = adw::PreferencesGroup::builder()
        .title(&t("Test Method"))
        .description(&t("Choose the component and stress algorithm. The badge shows the intensity level."))
        .build();

    let all_methods = [
        StressMethod::CpuMatrix,
        StressMethod::CpuPi,
        StressMethod::CpuAvx,
        StressMethod::RamPattern,
        StressMethod::RamVm,
        StressMethod::GpuGlmark2,
        StressMethod::GpuTessellation,
        StressMethod::DiskSeq,
        StressMethod::DiskRandom,
    ];

    // Build a selectable ListBox where each row has an intensity badge.
    let method_list = gtk::ListBox::new();
    method_list.add_css_class("boxed-list");
    method_list.set_selection_mode(gtk::SelectionMode::Single);

    // Track selected index in an Rc<Cell>.
    let selected_method_idx = std::rc::Rc::new(std::cell::Cell::new(0usize));

    for method in &all_methods {
        let row = adw::ActionRow::builder()
            .title(&method.label())
            .subtitle(&method.component().to_string())
            .activatable(true)
            .build();

        // Intensity badge label.
        let badge = gtk::Label::new(Some(&method.intensity_label()));
        badge.add_css_class("badge");
        badge.add_css_class(method.badge_css_class());
        badge.set_valign(gtk::Align::Center);
        row.add_suffix(&badge);

        method_list.append(&row);
    }

    // Select first row by default.
    if let Some(first) = method_list.row_at_index(0) {
        method_list.select_row(Some(&first));
    }

    let sel_idx_connect = std::rc::Rc::clone(&selected_method_idx);
    method_list.connect_row_selected(move |_, row| {
        if let Some(r) = row {
            sel_idx_connect.set(r.index() as usize);
        }
    });

    method_group.add(&method_list);
    body.append(&method_group);

    // ── Safety settings ───────────────────────────────────────────────────
    let safety_group = adw::PreferencesGroup::builder()
        .title(&t("Safety Watchdog"))
        .description(&t("Stress process is killed (SIGKILL) if temperature exceeds the limit."))
        .build();

    let temp_limit_row = adw::SpinRow::new(
        Some(&gtk::Adjustment::new(90.0, 50.0, 110.0, 1.0, 5.0, 0.0)),
        1.0,
        0,
    );
    temp_limit_row.set_title(&t("Temperature limit (°C)"));
    safety_group.add(&temp_limit_row);
    body.append(&safety_group);

    // ── Live status ───────────────────────────────────────────────────────
    let status_group = adw::PreferencesGroup::builder()
        .title(&t("Live Status"))
        .build();

    let cpu_temp_row = adw::ActionRow::builder()
        .title(&t("CPU Temperature"))
        .subtitle("—")
        .build();
    let gpu_temp_row = adw::ActionRow::builder()
        .title(&t("GPU Temperature"))
        .subtitle("—")
        .build();
    let elapsed_row = adw::ActionRow::builder()
        .title(&t("Elapsed"))
        .subtitle("—")
        .build();
    let cpu_freq_row = adw::ActionRow::builder()
        .title(&t("CPU Frequency"))
        .subtitle("—")
        .build();
    let mem_row = adw::ActionRow::builder()
        .title(&t("Available RAM"))
        .subtitle("—")
        .build();
    let load_row = adw::ActionRow::builder()
        .title(&t("Load Average (1m)"))
        .subtitle("—")
        .build();
    let status_row = adw::ActionRow::builder()
        .title(&t("Status"))
        .subtitle(&t("Idle"))
        .build();

    status_group.add(&cpu_temp_row);
    status_group.add(&gpu_temp_row);
    status_group.add(&elapsed_row);
    status_group.add(&cpu_freq_row);
    status_group.add(&mem_row);
    status_group.add(&load_row);
    status_group.add(&status_row);
    body.append(&status_group);

    // ── Control buttons ───────────────────────────────────────────────────
    let btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    btn_box.set_halign(gtk::Align::Center);

    let start_btn = gtk::Button::builder()
        .label(&t("Start Stress Test"))
        .css_classes(["suggested-action", "pill"])
        .build();
    let stop_btn = gtk::Button::builder()
        .label(&t("Stop"))
        .css_classes(["destructive-action", "pill"])
        .sensitive(false)
        .build();

    btn_box.append(&start_btn);
    btn_box.append(&stop_btn);
    body.append(&btn_box);

    // ── Report section ────────────────────────────────────────────────────
    let report_group = adw::PreferencesGroup::builder()
        .title(&t("Last Session Report"))
        .build();

    let report_peak_cpu = adw::ActionRow::builder()
        .title(&t("Peak CPU Temp"))
        .subtitle("—")
        .build();
    let report_peak_gpu = adw::ActionRow::builder()
        .title(&t("Peak GPU Temp"))
        .subtitle("—")
        .build();
    let report_throttle = adw::ActionRow::builder()
        .title(&t("Thermal Throttling"))
        .subtitle("—")
        .build();
    let report_energy = adw::ActionRow::builder()
        .title(&t("Energy Consumed"))
        .subtitle("—")
        .build();
    let report_power = adw::ActionRow::builder()
        .title(&t("Average Power"))
        .subtitle("—")
        .build();
    let report_duration = adw::ActionRow::builder()
        .title(&t("Duration"))
        .subtitle("—")
        .build();
    let report_method = adw::ActionRow::builder()
        .title(&t("Method"))
        .subtitle("—")
        .build();
    let report_exit = adw::ActionRow::builder()
        .title(&t("Exit Reason"))
        .subtitle("—")
        .build();
    let report_peak_freq = adw::ActionRow::builder()
        .title(&t("Peak CPU Frequency"))
        .subtitle("—")
        .build();
    let report_avg_freq = adw::ActionRow::builder()
        .title(&t("Average CPU Frequency"))
        .subtitle("—")
        .build();
    let report_mem_start = adw::ActionRow::builder()
        .title(&t("RAM at Start"))
        .subtitle("—")
        .build();
    let report_mem_end = adw::ActionRow::builder()
        .title(&t("RAM at End"))
        .subtitle("—")
        .build();
    let report_mem_min = adw::ActionRow::builder()
        .title(&t("Lowest Available RAM"))
        .subtitle("—")
        .build();
    let report_load = adw::ActionRow::builder()
        .title(&t("Load Average (1m)"))
        .subtitle("—")
        .build();
    let report_conclusion = adw::ActionRow::builder()
        .title(&t("Conclusion"))
        .subtitle("—")
        .build();

    report_group.add(&report_method);
    report_group.add(&report_peak_cpu);
    report_group.add(&report_peak_gpu);
    report_group.add(&report_throttle);
    report_group.add(&report_energy);
    report_group.add(&report_power);
    report_group.add(&report_duration);
    report_group.add(&report_peak_freq);
    report_group.add(&report_avg_freq);
    report_group.add(&report_mem_start);
    report_group.add(&report_mem_end);
    report_group.add(&report_mem_min);
    report_group.add(&report_load);
    report_group.add(&report_exit);
    report_group.add(&report_conclusion);
    body.append(&report_group);

    // ── Wire Start button ─────────────────────────────────────────────────
    let runner_start = Rc::clone(&runner);
    let start_btn_c = start_btn.clone();
    let stop_btn_c = stop_btn.clone();
    let status_row_c = status_row.clone();
    let sel_idx_start = std::rc::Rc::clone(&selected_method_idx);
    let temp_limit_c = temp_limit_row.clone();

    start_btn.connect_clicked(move |_| {
        let idx = sel_idx_start.get();
        let method = all_methods[idx.min(all_methods.len() - 1)];
        let temp_limit = temp_limit_c.value();

        match runner_start.start(method, temp_limit) {
            Ok(()) => {
                start_btn_c.set_sensitive(false);
                stop_btn_c.set_sensitive(true);
                status_row_c.set_subtitle(&t("Running…"));
            }
            Err(msg) => {
                let dialog = adw::AlertDialog::builder()
                    .heading(&t("Cannot Start Stress Test"))
                    .body(&msg)
                    .build();
                dialog.add_response("ok", &t("OK"));
                if let Some(root_widget) = start_btn_c.root() {
                    if let Ok(win) = root_widget.dynamic_cast::<gtk::Window>() {
                        dialog.present(&win);
                    }
                }
            }
        }
    });

    // ── Wire Stop button ──────────────────────────────────────────────────
    let runner_stop = Rc::clone(&runner);
    let start_btn_s = start_btn.clone();
    let stop_btn_s = stop_btn.clone();
    let status_row_s = status_row.clone();
    let report_peak_cpu_s = report_peak_cpu.clone();
    let report_peak_gpu_s = report_peak_gpu.clone();
    let report_throttle_s = report_throttle.clone();
    let report_energy_s = report_energy.clone();
    let report_power_s = report_power.clone();
    let report_duration_s = report_duration.clone();
    let report_method_s = report_method.clone();
    let report_exit_s = report_exit.clone();
    let report_peak_freq_s = report_peak_freq.clone();
    let report_avg_freq_s = report_avg_freq.clone();
    let report_mem_start_s = report_mem_start.clone();
    let report_mem_end_s = report_mem_end.clone();
    let report_mem_min_s = report_mem_min.clone();
    let report_load_s = report_load.clone();
    let report_conclusion_s = report_conclusion.clone();

    stop_btn.connect_clicked(move |_| {
        if let Some(report) = runner_stop.stop() {
            update_report_ui(
                &report_method_s,
                &report_peak_cpu_s,
                &report_peak_gpu_s,
                &report_throttle_s,
                &report_energy_s,
                &report_power_s,
                &report_duration_s,
                &report_exit_s,
                &report_peak_freq_s,
                &report_avg_freq_s,
                &report_mem_start_s,
                &report_mem_end_s,
                &report_mem_min_s,
                &report_load_s,
                &report_conclusion_s,
                &report,
            );
        }
        status_row_s.set_subtitle(&t("Idle (stopped manually)"));
        start_btn_s.set_sensitive(true);
        stop_btn_s.set_sensitive(false);
    });

    // ── Periodic UI tick ─────────────────────────────────────────────────
    // All widget updates happen here on the main thread. The runner's watchdog
    // thread only writes to Arc<Mutex<RunnerState>>; this timer polls it.
    let runner_tick = Rc::clone(&runner);
    let cpu_temp_tick = cpu_temp_row.clone();
    let gpu_temp_tick = gpu_temp_row.clone();
    let elapsed_tick = elapsed_row.clone();
    let cpu_freq_tick = cpu_freq_row.clone();
    let mem_tick = mem_row.clone();
    let load_tick = load_row.clone();
    let status_tick = status_row.clone();
    let start_btn_tick = start_btn.clone();
    let stop_btn_tick = stop_btn.clone();
    let report_peak_cpu_t = report_peak_cpu.clone();
    let report_peak_gpu_t = report_peak_gpu.clone();
    let report_throttle_t = report_throttle.clone();
    let report_energy_t = report_energy.clone();
    let report_power_t = report_power.clone();
    let report_duration_t = report_duration.clone();
    let report_method_t = report_method.clone();
    let report_exit_t = report_exit.clone();
    let report_peak_freq_t = report_peak_freq.clone();
    let report_avg_freq_t = report_avg_freq.clone();
    let report_mem_start_t = report_mem_start.clone();
    let report_mem_end_t = report_mem_end.clone();
    let report_mem_min_t = report_mem_min.clone();
    let report_load_t = report_load.clone();
    let report_conclusion_t = report_conclusion.clone();

    glib::timeout_add_seconds_local(1, move || {
        let (cpu, gpu) = runner_tick.live_temps();
        cpu_temp_tick.set_subtitle(&format!("{cpu:.1} °C"));
        gpu_temp_tick.set_subtitle(&format!("{gpu:.1} °C"));
        if let Some(freq) = read_cpu_freq_mhz() {
            cpu_freq_tick.set_subtitle(&format!("{freq:.0} MHz"));
        }
        if let Some(mem_kb) = read_meminfo_field("MemAvailable") {
            mem_tick.set_subtitle(&format!("{} MB", mem_kb / 1024));
        }
        if let Some(load_1m) = read_loadavg_1m() {
            load_tick.set_subtitle(&format!("{load_1m:.2}"));
        }

        if runner_tick.is_running() {
            let elapsed = runner_tick.elapsed_secs();
            let mins = elapsed / 60;
            let secs = elapsed % 60;
            elapsed_tick.set_subtitle(&format!("{mins}m {secs:02}s"));
            status_tick.set_subtitle(&t("Running…"));
        }

        // Poll for a completed report (natural finish or watchdog kill).
        if let Some(report) = runner_tick.take_pending_report() {
            update_report_ui(
                &report_method_t,
                &report_peak_cpu_t,
                &report_peak_gpu_t,
                &report_throttle_t,
                &report_energy_t,
                &report_power_t,
                &report_duration_t,
                &report_exit_t,
                &report_peak_freq_t,
                &report_avg_freq_t,
                &report_mem_start_t,
                &report_mem_end_t,
                &report_mem_min_t,
                &report_load_t,
                &report_conclusion_t,
                &report,
            );
            let msg = if report.stopped_by_watchdog {
                t("Idle (watchdog: temperature limit reached)")
            } else {
                t("Idle (process exited)")
            };
            status_tick.set_subtitle(&msg);
            start_btn_tick.set_sensitive(true);
            stop_btn_tick.set_sensitive(false);
        }

        glib::ControlFlow::Continue
    });

    root
}

// ────────────────────────────────────────────────────────────────────────────
// Helper: update the report rows in the UI.
// ────────────────────────────────────────────────────────────────────────────

fn update_report_ui(
    method: &adw::ActionRow,
    peak_cpu: &adw::ActionRow,
    peak_gpu: &adw::ActionRow,
    throttle: &adw::ActionRow,
    energy: &adw::ActionRow,
    power: &adw::ActionRow,
    duration: &adw::ActionRow,
    exit_reason: &adw::ActionRow,
    peak_freq: &adw::ActionRow,
    avg_freq: &adw::ActionRow,
    mem_start: &adw::ActionRow,
    mem_end: &adw::ActionRow,
    mem_min: &adw::ActionRow,
    load: &adw::ActionRow,
    conclusion: &adw::ActionRow,
    report: &StressReport,
) {
    let method_label = report
        .method
        .map(|m| m.label())
        .unwrap_or_else(|| t("Unknown"));
    method.set_subtitle(&method_label);

    peak_cpu.set_subtitle(&format!("{:.1} °C", report.peak_cpu_temp_c));
    peak_gpu.set_subtitle(&format!("{:.1} °C", report.peak_gpu_temp_c));

    let throttle_text = if report.thermal_throttling_detected {
        t("⚠ Throttling detected")
    } else {
        t("No throttling")
    };
    throttle.set_subtitle(&throttle_text);

    if let Some(j) = report.energy_joules() {
        energy.set_subtitle(&format!("{j:.2} J"));
    } else {
        energy.set_subtitle(&t("N/A (RAPL not available)"));
    }

    if let Some(w) = report.average_power_w() {
        power.set_subtitle(&format!("{w:.1} W"));
    } else {
        power.set_subtitle(&t("N/A"));
    }

    let mins = report.duration_secs / 60;
    let secs = report.duration_secs % 60;
    duration.set_subtitle(&format!("{mins}m {secs:02}s"));

    if report.peak_cpu_freq_mhz > 0.0 {
        peak_freq.set_subtitle(&format!("{:.0} MHz", report.peak_cpu_freq_mhz));
    } else {
        peak_freq.set_subtitle(&t("N/A"));
    }

    if let Some(v) = report.avg_cpu_freq_mhz {
        avg_freq.set_subtitle(&format!("{v:.0} MHz"));
    } else {
        avg_freq.set_subtitle(&t("N/A"));
    }

    if let Some(v) = report.start_mem_available_mb {
        mem_start.set_subtitle(&format!("{v} MB"));
    } else {
        mem_start.set_subtitle(&t("N/A"));
    }
    if let Some(v) = report.end_mem_available_mb {
        mem_end.set_subtitle(&format!("{v} MB"));
    } else {
        mem_end.set_subtitle(&t("N/A"));
    }
    if let Some(v) = report.min_mem_available_mb {
        mem_min.set_subtitle(&format!("{v} MB"));
    } else {
        mem_min.set_subtitle(&t("N/A"));
    }

    if let Some(v) = report.load_avg_1m {
        load.set_subtitle(&format!("{v:.2}"));
    } else {
        load.set_subtitle(&t("N/A"));
    }

    let reason = if report.stopped_by_watchdog {
        t("Stopped by watchdog (temperature limit)")
    } else if report.stopped_manually {
        t("Stopped manually")
    } else if let Some(code) = report.exit_code {
        format!("{} {code}", t("Process exited with code"))
    } else {
        t("Process finished")
    };
    exit_reason.set_subtitle(&reason);

    let conclusion_text = if report.stopped_by_watchdog {
        t("Result: FAIL - thermal limit reached during stress.")
    } else if report.thermal_throttling_detected {
        t("Result: WARNING - throttling detected, cooling may be insufficient.")
    } else if report.duration_secs < 10 {
        t("Result: INCONCLUSIVE - test duration too short.")
    } else {
        t("Result: PASS - sustained load completed without thermal protection.")
    };
    conclusion.set_subtitle(&conclusion_text);
}
