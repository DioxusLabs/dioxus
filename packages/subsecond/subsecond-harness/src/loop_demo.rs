pub fn launch() {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        subsecond::call(|| tick());
    }
}

fn tick() {
    println!("boom!12312312");
}
