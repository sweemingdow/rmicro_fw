use local_ip_address::local_ip;

// 获取本机ip
pub fn get_machine_ip() -> String {
    local_ip()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}
