use std::fs;
use std::thread;
use std::time::Duration;

use linux_hw_monitor::config::AppConfig;
use linux_hw_monitor::sensors::manager::SensorManager;
use tempfile::TempDir;

#[test]
fn manager_produces_updates() {
    let tmp = TempDir::new().expect("tempdir");
    let sys = tmp.path().join("sys");
    let proc = tmp.path().join("proc");

    fs::create_dir_all(sys.join("class/hwmon/hwmon0")).expect("mk hwmon");
    fs::create_dir_all(sys.join("class/net/eth0")).expect("mk net");
    fs::create_dir_all(sys.join("block/sda/device")).expect("mk block");
    fs::create_dir_all(proc.join("net")).expect("mk proc net");

    fs::write(sys.join("class/hwmon/hwmon0/name"), "coretemp\n").expect("write hwmon name");
    fs::write(sys.join("class/hwmon/hwmon0/temp1_input"), "55000\n").expect("write temp");
    fs::write(proc.join("stat"), "cpu 1 1 1 1 1 1 1 1 1 1\ncpu0 1 1 1 1 1 1 1 1 1 1\n").expect("write stat");
    fs::write(proc.join("meminfo"), "MemTotal: 1000 kB\nMemAvailable: 500 kB\n").expect("write meminfo");
    fs::write(proc.join("diskstats"), "8 0 sda 10 0 100 0 5 0 200 0 0 0 0 0\n").expect("write diskstats");
    fs::write(proc.join("net/dev"), "Inter-|   Receive | Transmit\n face |bytes packets errs drop fifo frame compressed multicast|bytes packets errs drop fifo colls carrier compressed\neth0: 100 0 0 0 0 0 0 0 200 0 0 0 0 0 0 0\n").expect("write net dev");
    fs::write(sys.join("class/net/eth0/operstate"), "up\n").expect("write operstate");
    fs::write(sys.join("class/net/eth0/speed"), "1000\n").expect("write speed");
    fs::write(sys.join("block/sda/device/model"), "MockDisk\n").expect("write model");
    fs::write(sys.join("block/sda/device/vendor"), "MockVendor\n").expect("write vendor");

    let manager = SensorManager::new();
    let mut cfg = AppConfig::default();
    cfg.path_config.sys_root = sys;
    cfg.path_config.proc_root = proc;
    manager.start(cfg);

    thread::sleep(Duration::from_secs(2));
    let data = manager.rx.borrow().clone();
    assert!(!data.timestamp.is_empty());
    assert!(!data.groups.is_empty());
}
