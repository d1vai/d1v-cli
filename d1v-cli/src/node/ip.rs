use super::IpArgs;
use super::ingress;
use crate::Context;
use crate::error::{Error, Result};
use anyhow::anyhow;

pub async fn run(_ctx: &Context, args: IpArgs) -> Result<()> {
    eprintln!("🌐 正在探测公网 IP...");
    let ip = ingress::detect_public_ip().await.ok_or_else(|| {
        Error::Other(anyhow!(
            "无法探测公网 IP，请检查网络连接（使用 api.ipify.org / ifconfig.me）"
        ))
    })?;

    if args.json {
        println!("{}", serde_json::json!({ "ip": ip }));
    } else {
        eprintln!("✅ 公网 IP: {}", ip);
        println!("{}", ip);
    }
    Ok(())
}
