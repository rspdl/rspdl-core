use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture {
    directory: PathBuf,
    alpha: PathBuf,
    beta: PathBuf,
    data: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("rspdl-multi-file-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&directory).expect("fixture directory should be created");
        let alpha = directory.join("alpha.rspdl");
        let beta = directory.join("beta.rspdl");
        let data = directory.join("data.json");
        fs::write(
            &alpha,
            "@모듈 알파(alpha)\n항목(item)은 다음 필드들로 구성되어 있다.\n    값(value): 필수 정수\n항목의 값은 0보다 커야 한다.\n",
        )
        .expect("alpha source should be written");
        fs::write(
            &beta,
            "@모듈 베타(beta)\n항목(item)은 다음 필드들로 구성되어 있다.\n    값(value): 필수 정수\n항목의 값은 10보다 작아야 한다.\n",
        )
        .expect("beta source should be written");
        fs::write(
            &data,
            r#"{
              "records": {
                "alpha.item": [{"$id": "alpha-1", "value": 1}],
                "beta.item": [{"$id": "beta-1", "value": 9}]
              }
            }"#,
        )
        .expect("runtime data should be written");
        Self {
            directory,
            alpha,
            beta,
            data,
        }
    }

    fn run(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rspdl"))
            .args(arguments)
            .output()
            .expect("rspdl command should run")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn compile_accepts_multiple_files_and_canonicalizes_their_order() {
    let fixture = Fixture::new();
    let alpha = fixture.alpha.to_str().unwrap();
    let beta = fixture.beta.to_str().unwrap();

    let forward = fixture.run(&["compile", alpha, beta, "--json"]);
    let reverse = fixture.run(&["compile", beta, alpha, "--json"]);

    assert!(forward.status.success(), "{:?}", forward);
    assert!(reverse.status.success(), "{:?}", reverse);
    assert_eq!(forward.stdout, reverse.stdout);
    let output: serde_json::Value = serde_json::from_slice(&forward.stdout).unwrap();
    assert_eq!(output["files"].as_array().unwrap().len(), 2);
}

#[test]
fn check_validates_one_runtime_input_against_every_file() {
    let fixture = Fixture::new();
    let output = fixture.run(&[
        "check",
        fixture.alpha.to_str().unwrap(),
        fixture.beta.to_str().unwrap(),
        "--data",
        fixture.data.to_str().unwrap(),
        "--json",
    ]);

    assert!(output.status.success(), "{:?}", output);
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["compilation"]["files"].as_array().unwrap().len(), 2);
    assert!(report["runtime_diagnostics"].as_array().unwrap().is_empty());
}
