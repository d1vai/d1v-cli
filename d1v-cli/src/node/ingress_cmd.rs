use super::ingress;
use super::{IngressCommand, IngressConfigureArgs, IngressDetectArgs};
use crate::Context;
use crate::error::{Error, Result};
use anyhow::anyhow;

pub async fn run(_ctx: &Context, command: IngressCommand) -> Result<()> {
    match command {
        IngressCommand::Detect(args) => run_detect(args).await,
        IngressCommand::Configure(args) => run_configure(args).await,
    }
}

async fn run_detect(args: IngressDetectArgs) -> Result<()> {
    eprintln!(
        "🔍 正在探测反向代理 ingress（端口 {}）...",
        args.agent_port
    );

    let detected = ingress::detect_public_ingress(
        args.agent_port,
        args.provider.as_deref(),
        args.hostname.as_deref(),
    )?;

    match detected {
        Some(candidate) => {
            let origin = candidate.origin();
            if args.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "detected": true,
                        "provider": candidate.provider,
                        "hostname": candidate.hostname,
                        "scheme": candidate.scheme,
                        "external_port": candidate.external_port,
                        "upstream_host": candidate.upstream_host,
                        "upstream_port": candidate.upstream_port,
                        "config_path": candidate.config_path,
                        "confidence": candidate.confidence,
                        "origin": origin,
                    })
                );
            } else {
                eprintln!("✅ 检测到 {} ingress:", candidate.provider);
                eprintln!("   域名:       {}", candidate.hostname);
                eprintln!("   Origin:     {}", origin);
                eprintln!(
                    "   上游:       {}:{}",
                    candidate.upstream_host, candidate.upstream_port
                );
                eprintln!("   置信度:     {}%", candidate.confidence);
                eprintln!("   配置路径:   {}", candidate.config_path);
                println!("{}", origin);
            }
        }
        None => {
            if args.json {
                println!("{}", serde_json::json!({ "detected": false }));
            } else {
                eprintln!("ℹ️  未检测到已配置的反向代理 ingress");
                eprintln!("   可使用 'd1v node ingress configure <hostname>' 配置新的 ingress");
            }
        }
    }

    Ok(())
}

async fn run_configure(args: IngressConfigureArgs) -> Result<()> {
    eprintln!(
        "🔧 正在为 {} 配置 ingress（代理到 127.0.0.1:{}）...",
        args.hostname, args.agent_port
    );

    let configured = ingress::configure_public_ingress(
        args.agent_port,
        args.provider.as_deref(),
        &args.hostname,
    )?;

    match configured {
        Some(result) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "configured": true,
                        "provider": result.provider,
                        "hostname": result.hostname,
                        "config_path": result.config_path,
                    })
                );
            } else {
                eprintln!("✅ Ingress 配置成功:");
                eprintln!("   提供商:     {}", result.provider);
                eprintln!("   域名:       {}", result.hostname);
                eprintln!("   配置路径:   {}", result.config_path);
            }
        }
        None => {
            return Err(Error::Other(anyhow!(
                "未找到可配置的 ingress 提供商（需要 nginx 或 nginx-proxy-manager）"
            )));
        }
    }

    Ok(())
}
