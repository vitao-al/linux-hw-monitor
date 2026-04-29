use std::process::Command;

use serde_json::json;
use zbus::interface;

struct Helper;

#[interface(name = "io.github.usuario.LinuxHWMonitor.Helper")]
impl Helper {
    async fn get_smart_data(&self, device: &str) -> String {
        run_json_command("smartctl", &["-j", "-A", device])
    }

    async fn get_smart_health(&self, device: &str) -> String {
        run_json_command("smartctl", &["-j", "-H", device])
    }

    async fn get_memory_info(&self) -> String {
        run_json_command("dmidecode", &["-t", "memory", "-q"])
    }
}

fn run_json_command(cmd: &str, args: &[&str]) -> String {
    let output = Command::new(cmd).args(args).output();
    let Ok(out) = output else {
        return json!({"error": "command not available"}).to_string();
    };

    if out.status.success() {
        if let Ok(s) = String::from_utf8(out.stdout) {
            return s;
        }
        return json!({"error": "utf8 decode failed"}).to_string();
    }

    json!({
        "error": "command failed",
        "code": out.status.code(),
        "stderr": String::from_utf8(out.stderr).ok().unwrap_or_default()
    })
    .to_string()
}

#[tokio::main]
async fn main() -> zbus::Result<()> {
    let _connection = zbus::ConnectionBuilder::system()?
        .name("io.github.usuario.LinuxHWMonitor.Helper")?
        .serve_at("/io/github/usuario/LinuxHWMonitor/Helper", Helper)?
        .build()
        .await?;

    std::future::pending::<()>().await;
    Ok(())
}
