use clap::Parser;
use comrak::{markdown_to_html, ComrakOptions};
use devserver_lib;
use fs_extra;
use minijinja::{context, AutoEscape, Environment};
use serde::Serialize;
use std::ffi;
use std::fs;

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
    <ul class="nav">
    {% for item in items %}
      <li><a href="{{ item.path }}">{{ item.title }}</a></li>
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
    let args = Args::parse();
    let paths = fs::read_dir(&args.input_dir).unwrap();
    fs_extra::dir::create(&args.output_dir, true).expect("Cannot create output dir");

    let mut items = Vec::new();

    let mut options = ComrakOptions::default();
    options.extension.strikethrough = true;
    options.extension.tagfilter = false;
    options.render.unsafe_ = true;

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
            let html = markdown_to_html(&contents, &options);

            fs_extra::dir::create(
                format!("{}/{}", &args.output_dir, md_filename_sanitized),
                true,
            )
            .expect("Cannot create output dir");

            let c = contents.lines().count();
            for line in contents.lines().enumerate() {
                if line.0 == c-1 {
                    println!("{}", c.to_string());
                    println!("{}", &line.1);
                }
                if &line.1.len() > &3 && line.1.starts_with("![") {
                    let img_file_path = &line.1[4..&line.1.len() - 1];
                    println!(
                        "{}",
                        format!("{}/attachments/{}", &args.input_dir, &img_file_path)
                    );
                    fs::copy(
                        format!("{}/attachments/{}", &args.input_dir, &img_file_path),
                        format!(
                            "{}/{}/{}",
                            &args.output_dir, &md_filename_sanitized, &img_file_path
                        ),
                    )?;
                }
            }

            let template = template_env.get_template("single.html").unwrap();
            let result = template
                .render(context!(title => "Rabla!", body => &html))
                .unwrap();

            let path = format!("{}/{}/index.html", args.output_dir, md_filename_sanitized);
            fs::write(path, &result).expect("Unable to write file");

            let new_post = Item {
                title: title,
                path: format!("{}/", md_filename_sanitized),
            };
            items.push(new_post);
        }
    }

    // Write index.html & style.css
    template_env.add_template("index.html", TEMPLATE_INDEX).unwrap();
    let template = template_env.get_template("index.html").unwrap();

    let result = template
        .render(context!(title => "Rabla!", items => items))
        .unwrap();
    fs::write("site/index.html", &result).expect("Unable to write file");
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
