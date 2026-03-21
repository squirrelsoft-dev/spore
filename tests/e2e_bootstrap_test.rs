#![cfg(feature = "e2e")]

use std::path::Path;
use std::process::Command;

#[test]
#[ignore]
fn e2e_bootstrap_runs_successfully() {
    let workspace_root = env!("CARGO_MANIFEST_DIR");
    let script_path = Path::new(workspace_root).join("scripts/e2e-test.sh");

    assert!(
        script_path.exists(),
        "E2E test script not found at: {}",
        script_path.display()
    );

    let output = Command::new(&script_path)
        .current_dir(workspace_root)
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "Failed to execute E2E test script at {}: {e}",
                script_path.display()
            )
        });

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "E2E bootstrap test failed with exit code {:?}\n\n\
             --- stdout ---\n{stdout}\n\n--- stderr ---\n{stderr}",
            output.status.code()
        );
    }
}
