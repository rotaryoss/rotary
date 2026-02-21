use colored::Colorize;
use rotaryoss_core::{HealthReport, Playbook, SecretMetadata, Severity};

pub fn print_report(report: &HealthReport) {
    println!();
    println!("  {}", "ROTARY — Secret Health Report".bold());
    println!(
        "  {} · {} secrets · scanned just now",
        capitalize(&report.environment).dimmed(),
        report.total_secrets
    );
    println!();

    for check in &report.checks {
        let (icon, label) = match check.severity {
            Severity::Critical => ("●".red(), "CRITICAL".red().bold()),
            Severity::Warning => ("●".yellow(), "WARNING ".yellow().bold()),
            Severity::Ok => ("✓".green(), "OK      ".green()),
        };

        println!(
            "  {icon} {label}   {:<24} {}",
            check.key,
            check.reason.dimmed()
        );
    }

    println!();
    let score = report.score.0;
    let score_display = if score >= 80 {
        format!("{score}/100").green().bold()
    } else if score >= 50 {
        format!("{score}/100").yellow().bold()
    } else {
        format!("{score}/100").red().bold()
    };
    println!("  Health score: {score_display}");
    println!();
}

pub fn print_details(
    meta: &SecretMetadata,
    source_name: &str,
    environment: &str,
    playbook: Option<&Playbook>,
) {
    println!();
    println!("  {} {}", "ROTARY —".bold(), meta.key.bold());
    println!();

    // Metadata table
    println!("  {:<16} {}", "Source".dimmed(), source_name);
    println!("  {:<16} {}", "Environment".dimmed(), environment);
    println!(
        "  {:<16} {}",
        "Created".dimmed(),
        meta.created_at.format("%Y-%m-%d")
    );

    if let Some(rotated) = meta.last_rotated {
        let days = (chrono::Utc::now() - rotated).num_days();
        let rotated_str = format!("{} ({days} days ago)", rotated.format("%Y-%m-%d"));
        let display = if days > 90 {
            rotated_str.red()
        } else if days > 75 {
            rotated_str.yellow()
        } else {
            rotated_str.green()
        };
        println!("  {:<16} {display}", "Last rotated".dimmed());
    } else {
        println!("  {:<16} {}", "Last rotated".dimmed(), "unknown".yellow());
    }

    if let Some(accessed) = meta.last_accessed {
        println!(
            "  {:<16} {}",
            "Last accessed".dimmed(),
            accessed.format("%Y-%m-%d")
        );
    }

    match &meta.owner {
        Some(owner) => println!("  {:<16} {owner}", "Owner".dimmed()),
        None => println!("  {:<16} {}", "Owner".dimmed(), "unassigned".yellow()),
    }

    // Playbook section
    println!();
    if let Some(pb) = playbook {
        println!(
            "  {} {}",
            "Rotation playbook:".bold(),
            pb.playbook.description.dimmed()
        );
        println!();
        for (i, step) in pb.steps.iter().enumerate() {
            let num = format!("  {}.", i + 1);
            let action = format!("[{}]", step.action);
            println!("  {num} {:<12} {}", action.cyan(), step.description);
        }
    } else {
        println!("  {}", "No matching rotation playbook found.".dimmed());
        println!(
            "  {}",
            "Add one in playbooks/ with a matching pattern.".dimmed()
        );
    }
    println!();
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}
