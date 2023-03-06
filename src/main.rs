use std::fs;
use std::ffi;
use fs_extra;
use comrak::{markdown_to_html, ComrakOptions};
use clap::Parser;
use devserver_lib;
use minijinja::{context, Environment};

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
    port: Option<u32>
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
    return result.to_lowercase()
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

    for path in paths {
        let md_filepath = path.as_ref().unwrap().path(); //  blog/file?a.md
        let is_file = path.as_ref().unwrap().file_type().unwrap().is_file();

        if is_file && md_filepath.extension() == Some(ffi::OsStr::new("md")) {
            let md_filename = path.as_ref().unwrap().file_name().into_string().unwrap(); // file?a.md
            let title = md_filename[..&md_filename.len()-3].to_owned(); // file?a
            let md_filename_sanitized = sanitize_filename(&title);  // file_a

            let contents = fs::read_to_string(md_filepath)
                .expect("Should have been able to read the file");
            let html = markdown_to_html(&contents, &options);

            fs_extra::dir::create(format!("{}/{}", &args.output_dir, md_filename_sanitized), true)
                .expect("Cannot create output dir");

            for line in contents.lines() {
                if &line.len() > &3 && &line[..2] == "![" {
                    let img_file_path = &line[4..&line.len()-1];
                    println!("{}", format!("{}/attachments/{}", &args.input_dir, &img_file_path));
                    fs::copy(
                        format!("{}/attachments/{}", &args.input_dir, &img_file_path),
                        format!("{}/{}/{}", &args.output_dir, &md_filename_sanitized, &img_file_path)
                    )?;
                }
            }

            let path = format!("{}/{}/index.html", args.output_dir, md_filename_sanitized);
            fs::write(path, &html).expect("Unable to write file");

            let new_post = Item {
                title,
                path: md_filename_sanitized
            };
            items.push(new_post);
        }

    }

    // Write index.html
    let mut env = Environment::new();
    env.add_template("hello.txt", "Hello {{ name }}!").unwrap();
    let template = env.get_template("hello.txt").unwrap();

    let mut index_html_list = Vec::new();
    for item in &items {
        index_html_list.push(format!("<li><a href=\"{}/\">{}</a></li>", item.path.clone(), item.title.clone()));
    }
    let joined = index_html_list.join("\n");
    let result = template.render(context!(name => joined)).unwrap();
    fs::write("site/index.html", &result).expect("Unable to write file");


    if args.serve {
        let mut port: u32 = 8090;
        if args.port.is_some() {
            port = args.port.unwrap();
        }
        println!("\nServing at http://localhost:{}", port);
        devserver_lib::run(&"localhost", port, &"site", true, "");
    }

    Ok(())
}