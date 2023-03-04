use std::fs;
use std::ffi;
use fs_extra;
use comrak::{markdown_to_html, ComrakOptions};
use clap::Parser;

// fn gen_output_filename(dir_entry: &std::fs::DirEntry) -> std::ffi::OsString {
//     return dir_entry.file_name();
// }

struct Item {
    title: String,
    path: String,
    // contents: String
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input_dir: String,

    #[arg(short, long)]
    output_dir: String,
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
    return result;
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let paths = fs::read_dir(args.input_dir).unwrap();
    fs_extra::dir::create(&args.output_dir, true).expect("Cannot create output dir");

    let mut items = Vec::new();

    let mut options = ComrakOptions::default();
    options.extension.strikethrough = true;
    options.extension.tagfilter = false;
    options.render.unsafe_ = true;

    for path in paths {
        // let dir_entry = ;

        let mut filename: String = path.as_ref().unwrap().file_name().into_string().unwrap();
        let final_path = sanitize_filename(&filename);
        
        let filepath = path.as_ref().unwrap().path();

        let is_file = path.as_ref().unwrap().file_type().unwrap().is_file();
        if is_file && filepath.extension() == Some(ffi::OsStr::new("md")) {
            filename.truncate(filename.len() - 3);

            let contents = fs::read_to_string(filepath)
                .expect("Should have been able to read the file");
            // let html : String = markdown::to_html(&contents);
            let html = markdown_to_html(&contents, &options);
            
            fs_extra::dir::create(format!("{}/{}", &args.output_dir, final_path), true)
                .expect("Cannot create output dir");

            let path = format!("{}/{}/index.html", args.output_dir, final_path);
            fs::write(path, &html).expect("Unable to write file");

            let new_post = Item {
                title: filename,
                path: final_path,
                // contents: html
            };
            items.push(new_post);
        }

    }

    // Write index.html
    let mut index_html_list = Vec::new();
    for item in &items {
        index_html_list.push(format!("<li><a href=\"{}\">{}</a></li>", item.path.clone(), item.title.clone()));
    }
    let joined = index_html_list.join("\n");
    fs::write("site/index.html", &joined).expect("Unable to write file");

    println!("{}", joined);
    Ok(())
}