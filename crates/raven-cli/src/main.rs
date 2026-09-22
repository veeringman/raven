use raven_core::{principles, GoalState, Phase, PolicyDecision};
use raven_runtime::RunReport;

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None | Some("help") | Some("-h") | Some("--help") => {
            print_help();
        }
        Some("version") | Some("--version") => {
            println!("raven {}", env!("CARGO_PKG_VERSION"));
        }
        Some("principles") => {
            for (index, (title, body)) in principles().iter().enumerate() {
                println!("{:2}. {title}", index + 1);
                println!("    {body}");
            }
        }
        Some("demo") => {
            let name = args.next().unwrap_or_else(|| "prepare-my-day".to_string());
            if name != "prepare-my-day" {
                eprintln!("unknown demo '{name}'. try: raven demo prepare-my-day");
                std::process::exit(2);
            }
            let report = match args.next().as_deref() {
                None => raven_runtime::prepare_my_day(),
                Some("--approve") => raven_runtime::prepare_my_day_with_approval(),
                Some(other) => {
                    eprintln!("unknown demo option '{other}'. try: --approve");
                    std::process::exit(2);
                }
            };
            print_report(&report);
        }
        Some(other) => {
            eprintln!("unknown command '{other}'");
            print_help();
            std::process::exit(2);
        }
    }
}

fn print_help() {
    println!(
        "\
RAVEN — from intent to action

Usage:
  raven demo [prepare-my-day] [--approve]
                              Run the canonical demonstration
                              --approve resumes after AskUser and finishes
  raven principles            Print the architectural principles
  raven version
  raven help"
    );
}

fn print_report(report: &RunReport) {
    println!("RAVEN  ·  from intent to action");
    println!();
    println!("Intent");
    println!("  {}", report.goal.intent);
    println!();
    for entry in &report.trace {
        println!("{}  {}", phase_label(entry.phase), entry.summary);
        if let Some(decision) = entry.decision {
            println!("       policy {}", decision_label(decision));
        }
        if let Some(verdict) = &entry.verification {
            let mark = if verdict.is_confirmed() {
                "verified"
            } else {
                "not verified"
            };
            println!("       {mark}");
        }
    }
    println!();
    println!("Outcome");
    println!("  {:?}", report.state);
    match report.state {
        GoalState::WaitingForUser => {
            if let Some(pending) = &report.pending {
                println!(
                    "  Waiting on '{}'. Resume with approve, deny, or cancel.",
                    pending.capability_id
                );
            } else {
                println!("  The next step waits for you. It was not executed.");
            }
        }
        GoalState::Completed => {
            println!("  Goal completed under policy and verification.");
        }
        GoalState::Cancelled => {
            println!("  Run cancelled. No further tools will run.");
        }
        GoalState::Failed => {
            println!("  Goal failed. Pending work was not completed.");
        }
        _ => {}
    }
}

fn phase_label(phase: Phase) -> &'static str {
    match phase {
        Phase::Perceive => "Perceive",
        Phase::Reason => "Reason  ",
        Phase::Plan => "Plan    ",
        Phase::Act => "Act     ",
        Phase::Verify => "Verify  ",
        Phase::Adapt => "Adapt   ",
    }
}

fn decision_label(decision: PolicyDecision) -> &'static str {
    match decision {
        PolicyDecision::Allow => "allow",
        PolicyDecision::AllowWithPolicy => "allow with policy",
        PolicyDecision::AskUser => "ask user",
        PolicyDecision::Deny => "deny",
    }
}
