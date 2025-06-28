
use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_empty_yaml_input() -> Result<()> {
    let dir = tempdir()?;
    let output_path = dir.path().to_path_buf();
    let input_path = dir.path().join("input.yaml");

    let yaml_input = "";
    fs::write(&input_path, yaml_input)?;

    let mut cmd = Command::cargo_bin("slate")?;
    cmd.arg("--from")
        .arg("yaml")
        .arg("--to")
        .arg("systemd")
        .arg(input_path)
        .arg("-o")
        .arg(output_path);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Input for Systemd resulted in no units to process."));

    dir.close()?;
    Ok(())
}

#[test]
fn test_invalid_yaml_input() -> Result<()> {
    let dir = tempdir()?;
    let output_path = dir.path().to_path_buf();
    let input_path = dir.path().join("input.yaml");

    let yaml_input = "this: is: not: valid: yaml";
    fs::write(&input_path, yaml_input)?;

    let mut cmd = Command::cargo_bin("slate")?;
    cmd.arg("--from")
        .arg("yaml")
        .arg("--to")
        .arg("systemd")
        .arg(input_path)
        .arg("-o")
        .arg(output_path);

    cmd.assert().failure();

    dir.close()?;
    Ok(())
}

#[test]
fn test_service_with_timer_snapshot() -> Result<()> {
    let dir = tempdir()?;
    let output_path = dir.path().to_path_buf();
    let input_path = dir.path().join("input.yaml");

    let yaml_input = r#"
git_obsidian:
  Unit:
    Description: "Syncs Obsidian"
  Service:
    ExecStart: "/bin/zsh -c '$Zdir/subshell/cron.sh; $SSdir/git_obsidian.zsh;'"
  Timer:
    OnCalendar: "*:7/15"
"#;
    fs::write(&input_path, yaml_input)?;

    let mut cmd = Command::cargo_bin("slate")?;
    cmd.arg("--from")
        .arg("yaml")
        .arg("--to")
        .arg("systemd")
        .arg(input_path)
        .arg("-o")
        .arg(output_path.clone());

    cmd.assert().success();

    let service_content = fs::read_to_string(output_path.join("git_obsidian.service"))?;
    let timer_content = fs::read_to_string(output_path.join("git_obsidian.timer"))?;

    insta::assert_snapshot!("service_with_timer_service", service_content);
    insta::assert_snapshot!("service_with_timer_timer", timer_content);

    dir.close()?;
    Ok(())
}
