use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = std::env::var("OUT_DIR").unwrap();

    generate_embedded_lua(&manifest_dir, &out_dir);
    generate_embedded_themes(&manifest_dir, &out_dir);
}

/// Scans config subdirectories (widgets/, utils/) for Lua modules.
/// Directories with init.lua get module name from the dir path.
/// Bare .lua files get module name from file stem.
fn generate_embedded_lua(manifest_dir: &str, out_dir: &str) {
    let config_dir = Path::new(manifest_dir).join("../config");

    let mut entries: Vec<(String, String)> = Vec::new();

    // Scan widgets/ — each subdirectory: init.lua => "widgets.<name>",
    // sibling .lua files => "widgets.<name>.<stem>". Sort so init.lua comes AFTER
    // other files (its code may require sibling modules).
    let widgets_dir = config_dir.join("widgets");
    if let Ok(dirs) = fs::read_dir(&widgets_dir) {
        let mut dirs: Vec<_> = dirs.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).collect();
        dirs.sort_by_key(|e| e.path());
        for dir in dirs {
            let widget_name = dir.file_name().to_str().unwrap().to_string();
            if let Ok(files) = fs::read_dir(dir.path()) {
                let mut files: Vec<_> = files.filter_map(|e| e.ok()).collect();
                files.sort_by(|a, b| {
                    let a_init = a.path().file_stem().and_then(|s| s.to_str()) == Some("init");
                    let b_init = b.path().file_stem().and_then(|s| s.to_str()) == Some("init");
                    (a_init, a.path()).cmp(&(b_init, b.path()))
                });
                for entry in files {
                    let path = entry.path();
                    if path.extension().map(|e| e == "lua").unwrap_or(false) {
                        let stem = path.file_stem().unwrap().to_str().unwrap();
                        let module = if stem == "init" {
                            format!("widgets.{}", widget_name)
                        } else {
                            format!("widgets.{}.{}", widget_name, stem)
                        };
                        let abs = fs::canonicalize(&path).unwrap().display().to_string().replace('\\', "/");
                        entries.push((module, abs));
                    }
                }
            }
        }
    }

    // Scan utils/ — init.lua becomes "utils", other .lua files become "utils.<stem>"
    // Sort so init.lua comes AFTER other files (its code may require sibling modules)
    let utils_dir = config_dir.join("utils");
    if let Ok(files) = fs::read_dir(&utils_dir) {
        let mut files: Vec<_> = files.filter_map(|e| e.ok()).collect();
        files.sort_by(|a, b| {
            let a_init = a.path().file_stem().unwrap().to_str().unwrap() == "init";
            let b_init = b.path().file_stem().unwrap().to_str().unwrap() == "init";
            (a_init, a.path()).cmp(&(b_init, b.path()))
        });
        for entry in files {
            let path = entry.path();
            if path.extension().map(|e| e == "lua").unwrap_or(false) {
                let stem = path.file_stem().unwrap().to_str().unwrap();
                let module = if stem == "init" {
                    "utils".to_string()
                } else {
                    format!("utils.{}", stem)
                };
                let abs = fs::canonicalize(&path).unwrap().display().to_string().replace('\\', "/");
                entries.push((module, abs));
            }
        }
    }

    let mut code = String::from("/// Auto-generated embedded Lua module sources\nconst EMBEDDED_LUA_MODULES: &[(&str, &str)] = &[\n");
    for (module, abs_path) in &entries {
        code.push_str(&format!("    (\"{}\", include_str!(\"{}\")),\n", module, abs_path));
    }
    code.push_str("];\n");

    fs::write(Path::new(out_dir).join("embedded_lua_modules.rs"), code).unwrap();
    println!("cargo:rerun-if-changed=../config/widgets");
    println!("cargo:rerun-if-changed=../config/utils");
}

fn generate_embedded_themes(manifest_dir: &str, out_dir: &str) {
    let themes_dir = Path::new(manifest_dir).join("../config/themes");
    // Read the themes/init.lua for default theme, and individual theme files for colors
    let init_path = themes_dir.join("init.lua");
    let init_content = fs::read_to_string(&init_path).expect("Failed to read themes/init.lua");

    // Concatenate all theme files so parse_themes can extract them
    let mut content = String::new();
    for name in &["dark", "light", "mint", "rose_pine"] {
        let path = themes_dir.join(format!("{}.lua", name));
        if path.exists() {
            let theme_src = fs::read_to_string(&path).unwrap();
            // Wrap in themes.NAME = { ... } format for the parser
            content.push_str(&format!("themes.{} = {{\n", name));
            for line in theme_src.lines() {
                let trimmed = line.trim();
                // Skip return statement and theme-level comments
                if trimmed.starts_with("return") || trimmed.starts_with("--") {
                    continue;
                }
                // Skip opening/closing braces from the return { ... }
                if trimmed == "{" || trimmed == "}" {
                    continue;
                }
                content.push_str(line);
                content.push('\n');
            }
            content.push_str("}\n\n");
        }
    }
    // Append init content so parse_default_theme can find `current`
    content.push_str(&init_content);

    let themes = parse_themes(&content);
    let default_theme = parse_default_theme(&content);

    let mut code = String::new();
    code.push_str("/// Auto-generated theme definitions from config/theme.lua\n\n");
    code.push_str(&format!(
        "const DEFAULT_THEME: &str = \"{}\";\n\n",
        default_theme
    ));

    // Generate ThemeColors struct
    code.push_str(
        r#"#[derive(Debug, Clone)]
pub struct ThemeColors {
    bg_primary: &'static str,
    bg_secondary: &'static str,
    bg_tertiary: &'static str,
    bg_card: &'static str,
    text_primary: &'static str,
    text_secondary: &'static str,
    text_muted: &'static str,
    text_accent: &'static str,
    accent_primary: &'static str,
    accent_secondary: &'static str,
    accent_success: &'static str,
    accent_warning: &'static str,
    accent_error: &'static str,
    border_primary: &'static str,
    border_accent: &'static str,
    card_radius: i64,
    border_width: i64,
}

"#,
    );

    // Generate theme constants
    for (name, props) in &themes {
        let const_name = format!("THEME_{}", name.to_uppercase());
        code.push_str(&format!(
            "const {}: ThemeColors = ThemeColors {{\n",
            const_name
        ));

        for field in &[
            "bg_primary",
            "bg_secondary",
            "bg_tertiary",
            "bg_card",
            "text_primary",
            "text_secondary",
            "text_muted",
            "text_accent",
            "accent_primary",
            "accent_secondary",
            "accent_success",
            "accent_warning",
            "accent_error",
            "border_primary",
            "border_accent",
        ] {
            let value = props.get(*field).map(|s| s.as_str()).unwrap_or("#000000");
            code.push_str(&format!("    {}: \"{}\",\n", field, value));
        }

        // Integer fields
        let card_radius = props
            .get("card_radius")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(12);
        let border_width = props
            .get("border_width")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(1);

        code.push_str(&format!("    card_radius: {},\n", card_radius));
        code.push_str(&format!("    border_width: {},\n", border_width));
        code.push_str("};\n\n");
    }

    // Generate theme names array
    code.push_str("const THEME_NAMES: &[&str] = &[");
    for name in themes.keys() {
        code.push_str(&format!("\"{}\", ", name));
    }
    code.push_str("];\n\n");

    // Generate get_theme function
    code.push_str("fn get_theme(name: &str) -> &'static ThemeColors {\n");
    code.push_str("    match name {\n");
    for name in themes.keys() {
        let const_name = format!("THEME_{}", name.to_uppercase());
        code.push_str(&format!("        \"{}\" => &{},\n", name, const_name));
    }
    code.push_str("        _ => &THEME_DARK,\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    fs::write(Path::new(out_dir).join("embedded_themes.rs"), code).unwrap();
    println!("cargo:rerun-if-changed=../config/themes");
}

/// Simple parser to extract theme definitions from theme.lua
fn parse_themes(content: &str) -> HashMap<String, HashMap<String, String>> {
    let mut themes: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current_theme: Option<String> = None;
    let mut current_props: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();

        // Match theme block start: themes.dark = {
        if line.starts_with("themes.") && line.contains(" = {") {
            // Save previous theme if any
            if let Some(name) = current_theme.take() {
                themes.insert(name, std::mem::take(&mut current_props));
            }

            // Extract theme name
            if let Some(name) = line
                .strip_prefix("themes.")
                .and_then(|s| s.split_whitespace().next())
            {
                current_theme = Some(name.to_string());
            }
        }
        // Match property: key = "value" or key = number
        else if current_theme.is_some() && line.contains(" = ") && !line.starts_with("--") {
            if let Some((key, value)) = line.split_once(" = ") {
                let key = key.trim();
                let value = value.trim().trim_end_matches(',');

                // Extract string value
                if value.starts_with('"') && value.ends_with('"') {
                    let value = value.trim_matches('"');
                    current_props.insert(key.to_string(), value.to_string());
                }
                // Extract number value
                else if let Ok(_) = value.parse::<i64>() {
                    current_props.insert(key.to_string(), value.to_string());
                }
            }
        }
        // Match block end
        else if line == "}" && current_theme.is_some() {
            if let Some(name) = current_theme.take() {
                themes.insert(name, std::mem::take(&mut current_props));
            }
        }
    }

    themes
}

/// Parse the default theme from theme.lua (the "current" field)
fn parse_default_theme(content: &str) -> String {
    for line in content.lines() {
        let line = line.trim();
        // Match: current = "light",
        if line.starts_with("current") && line.contains(" = ") {
            if let Some((_, value)) = line.split_once(" = ") {
                let value = value.trim().trim_end_matches(',');
                if value.starts_with('"') && value.ends_with('"') {
                    return value.trim_matches('"').to_string();
                }
            }
        }
    }
    "dark".to_string()
}


