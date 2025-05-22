use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use tera::{Context, Tera};

const DEFAULT_TEMPLATE: &str = include_str!("../templates/test.spec.ts");

pub fn generate_playwright_spec<P: AsRef<Path>>(
    filename: P,
    description: &str,
    steps: &[String],
) -> std::io::Result<()> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let user_template_path = exe_dir.join("templates").join("test.spec.ts");

    let mut tera = Tera::default();

    // Prøv at læse template fra disk, ellers brug indbygget
    let template_content = if user_template_path.exists() {
        append_log("custom template fundet");
        fs::read_to_string(&user_template_path)?
    } else {
        DEFAULT_TEMPLATE.to_string()
    };

    // Registrer templaten dynamisk
    tera.add_raw_template("test.spec.ts", &template_content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Tera error: {e}")))?;

    let mut context = Context::new();
    context.insert("description", description);
    context.insert("steps", steps);

    let content = tera.render("test.spec.ts", &context)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Render error: {e}")))?;

    fs::write(filename, content)
}

fn append_log(msg: &str) {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let log_path = exe_dir.join("specgen.log");
    let mut file = OpenOptions::new().create(true).append(true).open(log_path).unwrap();
    let _ = writeln!(file, "{}", msg);
}