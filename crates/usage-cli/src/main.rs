use clap::Parser;
use std::path::PathBuf;
use usage_core::{discover_codex, MonitorConfig, MonitorHandle, MonitorState, Status};

#[derive(Parser)]
#[command(version, about = "macOS Codex 사용량 모니터 — GUI 없이 조회")]
struct Args {
    #[arg(long)]
    watch: bool,
    #[arg(long)]
    json: bool,
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(5..=3600))]
    interval: u64,
    #[arg(long)]
    codex_path: Option<PathBuf>,
}
fn print_state(state: &MonitorState, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string(state).expect("serializable state")
        );
        return;
    }
    if let Some(snapshot) = &state.snapshot {
        println!(
            "Codex 사용량 · 조회 시각 Unix {}{}",
            snapshot.fetched_at,
            if state.stale {
                " · 오래된 데이터"
            } else {
                ""
            }
        );
        if snapshot.ordinary_usage_allowed == Some(false) {
            println!("서버 상태: 일반 사용 허용 안 됨");
        }
        println!(
            "남은 초기화: {}",
            snapshot
                .remaining_reset_count
                .map_or_else(|| "정보 없음".into(), |count| format!("{count}회"))
        );
        if let Some(expirations) = &snapshot.reset_credit_expirations {
            for (index, expires) in expirations.iter().enumerate() {
                println!(
                    "  초기화 {} 사용기한: {}",
                    index + 1,
                    expires.map_or_else(|| "없음".into(), |time| format!("Unix {time}"))
                );
            }
            if snapshot
                .remaining_reset_count
                .is_some_and(|count| count > expirations.len() as u64)
            {
                println!("  일부 초기화의 사용기한 정보가 제공되지 않았습니다.");
            }
        } else {
            println!("  초기화 사용기한: 정보 없음");
        }
        for b in &snapshot.buckets {
            println!("  {}", b.name.as_deref().unwrap_or(&b.id));
            for (label, w) in [("기본", &b.primary), ("추가", &b.secondary)] {
                if let Some(w) = w {
                    println!(
                        "    {label}: 사용 {:.1}% · 잔여 {:.1}% · 기간 {}분 · 초기화 Unix {}",
                        w.used_percent,
                        w.remaining_percent(),
                        w.window_duration_mins
                            .map_or_else(|| "정보 없음".into(), |v| v.to_string()),
                        w.resets_at
                            .map_or_else(|| "정보 없음".into(), |v| v.to_string())
                    );
                }
            }
        }
    }
    if let Some(message) = &state.message {
        eprintln!("{message}");
    }
}
#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args = Args::parse();
    let monitor = match MonitorHandle::start(MonitorConfig {
        codex_path: args.codex_path.unwrap_or_else(discover_codex),
        interval_secs: args.interval,
    }) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let mut states = monitor.subscribe();
    let mut success = true;
    loop {
        let current = states.borrow_and_update().clone();
        if current.status == Status::Ready || (args.watch && current.status != Status::Connecting) {
            print_state(&current, args.json);
            if !args.watch {
                break;
            }
        } else if !args.watch && current.status != Status::Connecting {
            eprintln!("{}", current.message.as_deref().unwrap_or("조회 실패"));
            success = false;
            break;
        }
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            changed = states.changed() => if changed.is_err() { success = false; break; },
        }
    }
    monitor.shutdown().await;
    if success {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}
