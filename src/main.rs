use clap::{CommandFactory, Parser};
use pkg_owner::{format_table, inventory, matches};
use std::io::{self, Write};

#[derive(Parser)]
#[command(version, about = "查询 macOS 软件由谁管理（只读）")]
struct Args {
    #[arg(short = 'l', long, conflicts_with_all = ["search", "application"], help = "列出已发现的软件与命令")]
    list: bool,
    #[arg(
        short = 's',
        long,
        value_name = "TEXT",
        conflicts_with = "application",
        help = "模糊搜索：子串及相似拼写"
    )]
    search: Option<String>,
    #[arg(help = "软件名、命令名或绝对路径")]
    application: Option<String>,
    #[arg(
        short = 'd',
        long,
        value_name = "PATH",
        help = "额外递归扫描目录，可重复"
    )]
    scan_dir: Vec<String>,
    #[arg(short = 'j', long, help = "JSON 输出，包含证据与扫描警告")]
    json: bool,
}
fn main() {
    let args = Args::parse();
    if !args.list && args.search.is_none() && args.application.is_none() {
        let _ = Args::command().print_help();
        println!();
        return;
    }
    let mut data = inventory::scan(&args.scan_dir, args.application.as_deref());
    if let Some(query) = args.search.as_ref().or(args.application.as_ref()) {
        data.results
            .retain(|r| matches(r, query, args.search.is_some()));
    }
    let code = i32::from(data.results.is_empty() && !args.list);
    let output = if args.json {
        serde_json::to_string_pretty(&data).expect("serializable inventory")
    } else {
        for warning in &data.warnings {
            eprintln!("扫描未完成: {warning}");
        }
        let mut output = format_table(&data.results);
        if data.results.is_empty() {
            output.push_str("\n未找到匹配项；可尝试 --search 或 --scan-dir。");
        }
        output
    };
    if let Err(e) = writeln!(io::stdout().lock(), "{output}") {
        if e.kind() != io::ErrorKind::BrokenPipe {
            eprintln!("输出失败: {e}");
            std::process::exit(1);
        }
        return;
    }
    std::process::exit(code);
}
