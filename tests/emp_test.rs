use whispra::protocol::envelope;

#[test]
fn execute_emp_code() {
    println!("--- 2. Test EMP anonym");
    let _ = envelope::emp();
}
