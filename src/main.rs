use clap::Parser;
use serde::{Deserialize, Serialize};

use git_digger::Repository;

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    #[arg(
        long,
        default_value_t = 0,
        help = "Limit the number of repos we process."
    )]
    limit: u32,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
#[serde(deny_unknown_fields)]
struct BookMeta {
    title: String,

    #[serde(deserialize_with = "from_url")]
    repo: Repository,
    folder: Option<String>,

    site: Option<String>,
    description: Option<String>,
    comment: Option<String>,

    book: Option<Book>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Book {
    title: String,
    src: Option<String>,
    language: Option<String>,

    #[serde(alias = "text-direction")]
    text_direction: Option<String>,
    multilingual: Option<bool>,
    authors: Vec<String>,
    description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BookToml {
    book: Book,
}

fn main() {
    env_logger::init();
    let args = Cli::parse();
    let mut errors = 0;

    let repos_dir = std::fs::canonicalize("repos").unwrap();

    let mut books = read_the_mdbooks_file();

    let mut count = 0;
    for book in &mut books {
        log::info!("book: {:?}", book);
        match book.repo.update_repository(&repos_dir, false) {
            Ok(_) => {}
            Err(err) => {
                log::error!("Error updating repo: {:?}", err);
                errors += 1;
                book.error = Some(format!("{:?}", err));
                continue;
            }
        }
        count += 1;
        if args.limit > 0 && count >= args.limit {
            break;
        }
    }

    log::info!("Start processing repos");
    let mut count = 0;
    for book in &mut books {
        log::info!("book: {:?}", book);
        count += 1;
        if args.limit > 0 && count >= args.limit {
            break;
        }
        let book_toml_file = if let Some(folder) = book.folder.clone() {
            book.repo.path(&repos_dir).join(folder).join("book.toml")
        } else {
            book.repo.path(&repos_dir).join("book.toml")
        };

        log::info!("book.toml: {:?}", book_toml_file);
        if !book_toml_file.exists() {
            log::error!("book.toml does not exist: {:?}", book_toml_file);
            errors += 1;
            book.error = Some("book.toml does not exist".to_string());
            continue;
        }

        let content = std::fs::read_to_string(&book_toml_file).unwrap();

        let data = match toml::from_str::<BookToml>(&content) {
            Ok(data) => data,
            Err(err) => {
                log::error!("Error parsing toml {book_toml_file:?}: {:?}", err);
                errors += 1;
                book.error = Some(format!("Error parsing toml {book_toml_file:?}: {:?}", err));
                continue;
            }
        };
        println!("{:?}", data);
    }

    // Go over all the cloned repos and check if they are still in the mdbooks.yaml file
    //list content of a directory
    //let path = PathBuf::from(repos_dir);
    //let entries = std::fs::read_dir(path).unwrap();
    //for entry in entries {
    //    let entry = entry.unwrap();
    //    let path = entry.path();
    //    println!("{:?}", path);

    //    std::process::exit(0);
    //}

    let mut index_md = String::from("# mdbooks\n\n");
    index_md += "| Title | Repo | Description | Comment | Error |\n";
    index_md += "|-------|------|-------------|---------|-------|\n";
    for book in books {
        index_md += format!(
            "| [{}]({}) | [repo]({}) | {} | {} | {} |\n",
            book.title,
            book.site.unwrap_or("".to_string()),
            book.repo.url(),
            book.description.unwrap_or("".to_string()),
            book.comment.unwrap_or("".to_string()),
            book.error.unwrap_or("".to_string())
        )
        .as_str();
    }
    std::fs::write("report/src/index.md", index_md).unwrap();

    if errors > 0 {
        log::error!("There were {errors} errors");
        std::process::exit(1);
    }
}

fn read_the_mdbooks_file() -> Vec<BookMeta> {
    let file = std::fs::read_to_string("mdbooks.yaml").unwrap();
    let books: Vec<BookMeta> = serde_yaml::from_str(&file).unwrap();
    books
}

use serde::de;

fn from_url<'de, D>(deserializer: D) -> Result<Repository, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let r = Repository::from_url(&s).unwrap();
    Ok(r)
}
