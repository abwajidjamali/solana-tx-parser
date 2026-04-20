use clap::Parser;
use colored::*;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;
use solana_transaction_status::{
    EncodedTransaction, UiMessage, UiTransactionEncoding, option_serializer::OptionSerializer,
};
use std::str::FromStr;

//  Known Programs

fn known_program(id: &str) -> &'static str {
    match id {
        "11111111111111111111111111111111" => "System Program",
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => "SPL Token Program",
        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJe1bq8" => "Associated Token Program",
        "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s" => "Metaplex Token Metadata",
        "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin" => "Serum DEX v3",
        "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX" => "Serum DEX v4",
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" => "Jupiter Aggregator v6",
        "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => "Orca Whirlpool",
        "RVKd61ztZW9GUwhRbbLoYVRE5Xf1B2tVscKqwZqXgEr" => "Raydium Swap",
        "ComputeBudget111111111111111111111111111111" => "Compute Budget Program",
        "Vote111111111111111111111111111111111111111h" => "Vote Program",
        "Stake11111111111111111111111111111111111111" => "Stake Program",
        _ => "Unknown Program",
    }
}

//  CLI Args

/// Solana Transaction Parser — fetch and decode any Solana transaction
#[derive(Parser)]
#[command(name = "solana-tx-parser")]
#[command(about = "Fetch and decode raw Solana transactions from the terminal")]
struct Cli {
    /// Transaction signature to parse
    signature: String,

    /// RPC endpoint (default: Solana mainnet)
    #[arg(
        short = 'u',
        long,
        default_value = "https://api.mainnet-beta.solana.com"
    )]
    rpc: String,

    /// Show raw instruction data in base58
    #[arg(short = 'r', long)]
    raw: bool,
}

//  Helpers

fn sol(lamports: u64) -> String {
    format!("{:.9} SOL", lamports as f64 / 1_000_000_000.0)
}

fn short(pubkey: &str) -> String {
    if pubkey.len() > 16 {
        format!("{}...{}", &pubkey[..6], &pubkey[pubkey.len() - 4..])
    } else {
        pubkey.to_string()
    }
}

fn divider() {
    println!("{}", "═".repeat(60).cyan());
}

fn section(title: &str) {
    println!("\n{}", format!("▶ {}", title).yellow().bold());
    println!("{}", "─".repeat(60).dimmed());
}

//  Main

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    //  Connect to RPC
    println!("\n{}", "Connecting to Solana RPC...".dimmed());
    let client = RpcClient::new_with_commitment(cli.rpc.clone(), CommitmentConfig::confirmed());

    //  Parse signature
    let signature = match Signature::from_str(&cli.signature) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("{}", "Invalid transaction signature".red().bold());
            std::process::exit(1);
        }
    };

    //  Fetch transaction
    println!("{}", "Fetching transaction...".dimmed());
    let tx = match client.get_transaction(&signature, UiTransactionEncoding::JsonParsed) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{} {}", "Failed to fetch transaction:".red().bold(), e);
            std::process::exit(1);
        }
    };

    //  Header
    divider();
    println!("{}", "  SOLANA TRANSACTION PARSER".cyan().bold());
    divider();

    //  Overview
    section("OVERVIEW");

    println!("  {:<14} {}", "Signature:".bold(), cli.signature);
    println!("  {:<14} {}", "Network:".bold(), cli.rpc);

    // Status
    let meta = tx.transaction.meta.as_ref();
    let status = match meta {
        Some(m) => {
            if m.err.is_none() {
                "Success".green().bold().to_string()
            } else {
                format!("Failed — {:?}", m.err.as_ref().unwrap())
                    .red()
                    .bold()
                    .to_string()
            }
        }
        None => "Unknown".dimmed().to_string(),
    };
    println!("  {:<14} {}", "Status:".bold(), status);

    // Slot & timestamp
    println!(
        "  {:<14} {}",
        "Slot:".bold(),
        format!("{}", tx.slot).bright_white()
    );

    if let Some(ts) = tx.block_time {
        let dt = chrono_format(ts);
        println!("  {:<14} {}", "Timestamp:".bold(), dt);
    }

    // Fee
    if let Some(m) = meta {
        println!("  {:<14} {}", "Fee:".bold(), sol(m.fee).bright_yellow());

        // Compute units
        if let OptionSerializer::Some(units) = &m.compute_units_consumed {
            println!(
                "  {:<14} {}",
                "Compute:".bold(),
                format!("{} units", units).dimmed()
            );
        }
    }

    //  Accounts
    section("ACCOUNTS");

    let encoded_tx = &tx.transaction.transaction;
    match encoded_tx {
        EncodedTransaction::Json(ui_tx) => match &ui_tx.message {
            UiMessage::Parsed(msg) => {
                for (i, acc) in msg.account_keys.iter().enumerate() {
                    let mut flags = vec![];
                    if acc.signer {
                        flags.push("signer".cyan().to_string());
                    }
                    if acc.writable {
                        flags.push("writable".yellow().to_string());
                    }

                    let program_label = known_program(&acc.pubkey);
                    let label = if program_label != "Unknown Program" {
                        format!(" → {}", program_label).green().to_string()
                    } else {
                        String::new()
                    };

                    println!(
                        "  [{}] {}{}  {}",
                        format!("{}", i).dimmed(),
                        acc.pubkey.bright_white(),
                        label,
                        flags.join(", "),
                    );
                }
            }
            UiMessage::Raw(msg) => {
                for (i, key) in msg.account_keys.iter().enumerate() {
                    let label = known_program(key);
                    println!(
                        "  [{}] {} → {}",
                        i,
                        short(key).bright_white(),
                        label.green()
                    );
                }
            }
        },
        _ => println!("  {}", "Transaction encoding not supported".dimmed()),
    }

    //  Instructions
    section("INSTRUCTIONS");

    match encoded_tx {
        EncodedTransaction::Json(ui_tx) => {
            match &ui_tx.message {
                UiMessage::Parsed(msg) => {
                    for (i, ix) in msg.instructions.iter().enumerate() {
                        use solana_transaction_status::UiInstruction;
                        match ix {
                            UiInstruction::Parsed(parsed_ix) => {
                                use solana_transaction_status::UiParsedInstruction;
                                match parsed_ix {
                                    UiParsedInstruction::Parsed(detail) => {
                                        println!(
                                            "  {} {}",
                                            format!("[{}]", i).dimmed(),
                                            format!(
                                                "Program: {}",
                                                known_program(&detail.program_id)
                                            )
                                            .green()
                                            .bold()
                                        );
                                        println!(
                                            "      {:<12} {}",
                                            "Program ID:".bold(),
                                            detail.program_id.dimmed()
                                        );

                                        // Pretty-print parsed instruction info
                                        if let Some(obj) = detail.parsed.as_object() {
                                            if let Some(ix_type) = obj.get("type") {
                                                println!(
                                                    "      {:<12} {}",
                                                    "Type:".bold(),
                                                    ix_type.as_str().unwrap_or("?").bright_cyan()
                                                );
                                            }
                                            if let Some(info) = obj.get("info") {
                                                if let Some(info_obj) = info.as_object() {
                                                    for (key, val) in info_obj {
                                                        let display = match val {
                                                            serde_json::Value::String(s) => {
                                                                // Lamports -> SOL
                                                                if key == "lamports" {
                                                                    if let Ok(n) = s.parse::<u64>()
                                                                    {
                                                                        sol(n)
                                                                            .bright_yellow()
                                                                            .to_string()
                                                                    } else {
                                                                        s.clone()
                                                                    }
                                                                } else {
                                                                    s.bright_white().to_string()
                                                                }
                                                            }
                                                            serde_json::Value::Number(n) => {
                                                                if key == "lamports" {
                                                                    if let Some(v) = n.as_u64() {
                                                                        sol(v)
                                                                            .bright_yellow()
                                                                            .to_string()
                                                                    } else {
                                                                        n.to_string()
                                                                    }
                                                                } else {
                                                                    n.to_string()
                                                                        .bright_white()
                                                                        .to_string()
                                                                }
                                                            }
                                                            _ => val.to_string(),
                                                        };
                                                        println!(
                                                            "        {:<12} {}",
                                                            format!("{}:", key).bold(),
                                                            display
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    UiParsedInstruction::PartiallyDecoded(partial) => {
                                        println!(
                                            "  {} {}",
                                            format!("[{}]", i).dimmed(),
                                            format!(
                                                "Program: {}",
                                                known_program(&partial.program_id)
                                            )
                                            .green()
                                            .bold()
                                        );
                                        println!(
                                            "      {:<12} {}",
                                            "Program ID:".bold(),
                                            partial.program_id.dimmed()
                                        );
                                        println!(
                                            "      {:<12} {}",
                                            "Accounts:".bold(),
                                            partial
                                                .accounts
                                                .iter()
                                                .map(|a| short(a))
                                                .collect::<Vec<_>>()
                                                .join(", ")
                                                .bright_white()
                                        );
                                        if cli.raw {
                                            println!(
                                                "      {:<12} {}",
                                                "Data (b58):".bold(),
                                                partial.data.dimmed()
                                            );
                                        } else {
                                            println!(
                                                "      {:<12} {} {}",
                                                "Data:".bold(),
                                                format!(
                                                    "{} bytes",
                                                    bs58::decode(&partial.data)
                                                        .into_vec()
                                                        .map(|v| v.len())
                                                        .unwrap_or(0)
                                                )
                                                .dimmed(),
                                                "(use --raw to see full data)".dimmed()
                                            );
                                        }
                                    }
                                }
                            }
                            UiInstruction::Compiled(compiled) => {
                                println!(
                                    "  {} Program index: {}",
                                    format!("[{}]", i).dimmed(),
                                    compiled.program_id_index
                                );
                            }
                        }
                        if i < msg.instructions.len() - 1 {
                            println!("  {}", "·".repeat(40).dimmed());
                        }
                    }
                }
                _ => println!(
                    "  {}",
                    "Raw message format — use JsonParsed encoding".dimmed()
                ),
            }
        }
        _ => {}
    }

    //  Balance Changes
    if let Some(m) = meta {
        let pre = &m.pre_balances;
        let post = &m.post_balances;

        if !pre.is_empty() && pre.len() == post.len() {
            section("BALANCE CHANGES");

            // Collecting account keys for labels
            let keys: Vec<String> = match encoded_tx {
                EncodedTransaction::Json(ui_tx) => match &ui_tx.message {
                    UiMessage::Parsed(msg) => {
                        msg.account_keys.iter().map(|a| a.pubkey.clone()).collect()
                    }
                    UiMessage::Raw(msg) => msg.account_keys.clone(),
                },
                _ => vec![],
            };

            let mut any = false;
            for i in 0..pre.len() {
                let diff = post[i] as i64 - pre[i] as i64;
                if diff != 0 {
                    any = true;
                    let label = keys
                        .get(i)
                        .map(|k| short(k))
                        .unwrap_or_else(|| format!("Account {}", i));
                    let change = if diff > 0 {
                        format!("+{}", sol(diff as u64)).green().to_string()
                    } else {
                        format!("-{}", sol(diff.unsigned_abs())).red().to_string()
                    };
                    println!(
                        "  {}  {}  →  {}",
                        label.bright_white(),
                        sol(pre[i]).dimmed(),
                        change
                    );
                }
            }
            if !any {
                println!("  {}", "No balance changes".dimmed());
            }
        }
    }

    //  Footer
    println!();
    divider();
    println!("{}", "  github.com/abwajidjamali/solana-tx-parser".dimmed());
    divider();
    println!();
}

//  Timestamp formatter

fn chrono_format(unix: i64) -> String {
    // Manual UTC formatting without chrono dependency
    let secs = unix as u64;
    let mins = secs / 60;
    let hours = mins / 60;
    let days = hours / 24;

    let s = secs % 60;
    let m = mins % 60;
    let h = hours % 24;

    // Approximate date
    let years_approx = 1970 + days / 365;
    let day_of_year = days % 365;
    let month_approx = day_of_year / 30 + 1;
    let day_approx = day_of_year % 30 + 1;

    format!(
        "{}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        years_approx,
        month_approx.min(12),
        day_approx.min(31),
        h,
        m,
        s
    )
}
