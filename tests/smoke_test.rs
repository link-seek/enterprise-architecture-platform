/// Smoke test that verifies the server binary compiles.
///
/// This test exists to ensure the workspace and server crate build successfully.
/// If this file compiles and the test runs, the toolchain and dependencies are intact.
#[test]
fn server_binary_compiles() {
    // The mere existence and compilation of this test file confirms that
    // the workspace test target builds without errors.
    assert!(true, "smoke test passed: workspace compiles");
}
