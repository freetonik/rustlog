use clap::Parser;
use comrak::{markdown_to_html, ComrakOptions};
use devserver_lib;
use fs_extra;
use minijinja::{context, AutoEscape, Environment};
use serde::Serialize;
use std::ffi;
use std::fs;
use std::process::exit;

const TEMPLATE_SINGLE: &str = r#"<!doctype html>
<html>
<head>
    <title>{{ title }}</title>
    <link rel="stylesheet" href="/style.css">
    <meta name="description" content="Our first page">
    <meta name="keywords" content="html tutorial template">
</head>
<body>
    <main>
    <h1>{{ title }}</h1>
    {{ body }}
    </main>
</body>
</html>"#;

const TEMPLATE_INDEX: &str = r#"<!doctype html>
<html>
<head>
    <title>{{ title }}</title>
    <link rel="stylesheet" href="/style.css">
    <meta name="description" content="Our first page">
    <meta name="keywords" content="html tutorial template">
</head>
<body>
    <main>
    <h1>{{ title }}</h1>
    <ul class="nav">
    {% for item in items %}
      <li>{{ item.date }} <a href="{{ item.path }}">{{ item.title }}</a></li>
    {% endfor %}
    </ul>
    </main>
</body>
</html>"#;

const TEMPLATE_STYLE: &str = r#"
body {
  font-family: -apple-system,system-ui,BlinkMacSystemFont,"Segoe UI",Roboto,"Helvetica Neue",Arial,sans-serif;
  line-height: 1.4;
  font-size: 16px;
  margin: 0;
  background-image: radial-gradient(#53575712 1px, transparent 0);
  background-size: 8px 8px;
  background-color: #c0c7c842;
  border-top: 1rem solid #0000A8;
}

main {
  margin: 50px auto;
  max-width: 650px;
  padding: 16px;
}

img {
  max-width: 100%;
}

div.item {
  display: flex;
  margin-bottom: 0.5em;
}

div.date {
  margin-right: 1em;
}

@media screen and (max-width: 600px) {
  main {
    margin: 10px auto;
    max-width: 100%;
  }
}"#;

#[derive(Serialize)]
struct Item {
    title: String,
    path: String,
    date: String,
    date_internal: String
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input_dir: String,

    #[arg(short, long)]
    output_dir: String,

    #[arg(short, long)]
    serve: bool,

    #[arg(short, long)]
    port: Option<u32>,
}

fn extract_tags(tag_string: &str) -> Vec<String> {
    let tags: Vec<String> = tag_string
        .split_whitespace()
        .filter(|&s| s.starts_with('#'))
        .map(|s| s.trim_start_matches('#').to_string())
        .collect();
    tags
}

fn validate_dateline(line: &str) -> bool {
    let length_ok = line.len() == 10;
    let line_ok = line.chars().nth(2).unwrap() == '.' && line.chars().nth(5).unwrap() == '.';
    return length_ok && line_ok;
}

fn sanitize_filename(filename: &String) -> String {
    let allowed = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_".to_string();
    let mut result: String = "".to_string();
    for c in filename.chars() {
        if allowed.contains(c) {
            result.push(c);
        } else {
            result.push('_');
        }
    }
    return result
        .to_lowercase()
        .trim_start_matches('_')
        .trim_end_matches('_')
        .to_string();
}

fn main() -> std::io::Result<()> {
    println!(r#"
              _ _____ __    __
   ____ ___  (_) / (_) /_  / /___  ____ _
  / __ `__ \/ / / / / __ \/ / __ \/ __ `/
 / / / / / / / / / / /_/ / / /_/ / /_/ /
/_/ /_/ /_/_/_/_/_/_.___/_/\____/\__, /
                                /____/
    "#);
    let args = Args::parse();
    let paths = fs::read_dir(&args.input_dir).unwrap();
    fs_extra::dir::create(&args.output_dir, true).expect("Cannot create output dir");

    let mut items = Vec::new();

    let mut md_options = ComrakOptions::default();
    md_options.extension.strikethrough = true;
    md_options.extension.tagfilter = false;
    md_options.render.unsafe_ = true;

    let mut template_env = Environment::new();
    template_env.set_auto_escape_callback(|_name| AutoEscape::None);

    template_env
        .add_template("single.html", TEMPLATE_SINGLE)
        .unwrap();

    for path in paths {
        let md_filepath = path.as_ref().unwrap().path(); //  blog/file?a.md
        let is_file = path.as_ref().unwrap().file_type().unwrap().is_file();

        if is_file && md_filepath.extension() == Some(ffi::OsStr::new("md")) {
            let md_filename = path.as_ref().unwrap().file_name().into_string().unwrap(); // file?a.md
            let title = md_filename[..&md_filename.len() - 3].to_owned(); // file?a
            let md_filename_sanitized = sanitize_filename(&title); // file_a

            let contents =
                fs::read_to_string(md_filepath).expect("Should have been able to read the file");
            let html = markdown_to_html(&contents, &md_options);
            let mut date:String = "".to_string();
            let mut date_internal:String = "".to_string();

            fs_extra::dir::create(
                format!("{}/{}", &args.output_dir, md_filename_sanitized),
                true,
            )
            .expect("Cannot create output dir");

            let last_line_number = contents.lines().count() - 1;
            for line in contents.lines().enumerate() {
                let line_number = line.0;
                let line_content = line.1;

                // getting date
                if line_number == last_line_number {
                    if validate_dateline(&line_content) {
                        println!("{}", md_filename);
                    } else {
                        eprintln!("File '{}' does not contain date as DD.MM.YYYY on the last line", md_filename);
                        exit(1);
                    }
                    date = line_content.to_string();
                    date_internal = format!("{}{}{}", &line_content[6..10], &line_content[3..5], &line_content[0..2]);
                }
                // getting images
                if &line_content.len() > &6 && line_content.starts_with("![") {
                    let img_file_path = &line_content[4..&line_content.len() - 1];
                    fs::copy(
                        format!("{}/attachments/{}", &args.input_dir, &img_file_path),
                        format!(
                            "{}/{}/{}",
                            &args.output_dir, &md_filename_sanitized, &img_file_path
                        ),
                    )?;
                }
            }

            let single_template = template_env.get_template("single.html").unwrap();
            let rendered_html = single_template
                .render(context!(title => &title, body => &html))
                .unwrap();

            let path = format!("{}/{}/index.html", args.output_dir, md_filename_sanitized);
            fs::write(path, &rendered_html).expect("Unable to write file");

            items.push(Item {
                title,
                date: format!("{}/{}", &date[3..5], &date[8..10]),
                date_internal,
                path: format!("{}/", md_filename_sanitized),
            });
        }
    }

    items.sort_by(|a, b| b.date_internal.cmp(&a.date_internal));

    // Write index.html & style.css
    template_env.add_template("index.html", TEMPLATE_INDEX).unwrap();
    let index_template = template_env.get_template("index.html").unwrap();

    let rendered_html = index_template
        .render(context!(title => "Rakhim's blog", items => items))
        .unwrap();
    fs::write("site/index.html", &rendered_html).expect("Unable to write file");
    fs::write("site/style.css", TEMPLATE_STYLE).expect("Unable to write file");

    if args.serve {
        let mut port: u32 = 8090;
        if args.port.is_some() {
            port = args.port.unwrap();
        }
        println!("\nServing at http://localhost:{}", port);
        devserver_lib::run(&"localhost", port, &"site", false, "");
    }

    Ok(())
}

// TODO
// move from folder-based to .html based structure
// provide option to copy static files
// generate tag listing pages
