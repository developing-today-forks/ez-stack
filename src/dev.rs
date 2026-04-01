use anyhow::{Context, Result, bail};
use std::process::Command;

pub fn dev_port(branch: &str) -> u16 {
    let mut hash: u32 = 5381;
    for byte in branch.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u32);
    }
    10000 + (hash % 10000) as u16
}

pub fn terminate_listener_processes(port: u16) -> Result<Vec<u32>> {
    let pids = listener_pids(port)?;
    for pid in &pids {
        terminate_process(*pid)?;
    }
    Ok(pids)
}

fn listener_pids(port: u16) -> Result<Vec<u32>> {
    let output = Command::new("lsof")
        .args(["-nP", &format!("-iTCP:{port}"), "-sTCP:LISTEN", "-t"])
        .output()
        .with_context(|| format!("failed to run lsof for TCP port {port}"))?;

    if output.status.success() {
        return Ok(parse_pid_lines(&String::from_utf8_lossy(&output.stdout)));
    }

    if output.status.code() == Some(1) {
        return Ok(Vec::new());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    bail!("failed to query TCP port {port}: {stderr}");
}

fn terminate_process(pid: u32) -> Result<()> {
    let output = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .output()
        .with_context(|| format!("failed to terminate pid {pid}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    bail!("failed to terminate pid {pid}: {stderr}");
}

fn parse_pid_lines(output: &str) -> Vec<u32> {
    output
        .lines()
        .filter_map(|line| line.trim().parse::<u32>().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_port_is_stable() {
        assert_eq!(dev_port("feat/auth"), dev_port("feat/auth"));
        assert_ne!(dev_port("feat/auth"), dev_port("feat/api"));
    }

    #[test]
    fn parse_pid_lines_ignores_invalid_rows() {
        let parsed = parse_pid_lines("1234\nnoise\n5678\n");
        assert_eq!(parsed, vec![1234, 5678]);
    }
}
