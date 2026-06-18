use std::{io, path::PathBuf};

use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use clap::Parser;
use jiff::Zoned;
use rand::{SeedableRng, rngs::StdRng, seq::IndexedRandom};
use serde::Deserialize;

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(env)]
    key: String,

    #[arg(short, long, env)]
    pictures: PathBuf,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // build our application with a single route
    let app = Router::new()
        .route("/", get(serve_page))
        .route("/img", get(serve_image))
        .route("/img/{id}", get(serve_image_by_id))
        .route("/overview", get(serve_overview))
        .with_state(args);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Deserialize)]
struct QueryArgs {
    key: String,
}

async fn serve_overview(
    State(args): State<Args>,
    Query(query): Query<QueryArgs>,
) -> Result<Html<String>, StatusCode> {
    check_key(&args, &query)?;

    let image_paths = args.image_paths();
    let current_image_path = args.current_image_path();

    let current_element = image_paths
        .iter()
        .enumerate()
        .find(|(_, path)| **path == current_image_path)
        .map(|(id, _)| {
            format!(
                r#"
                    <div class="frame current" style="--frame-size: 20em; background-image: url('/img/{id}?key={}')"></div>
                    <hr>
                "#,
                args.key
            )
        })
        .unwrap_or_default();

    let image_elements = image_paths
        .into_iter()
        .enumerate()
        .map(|(id, path)| {
            format!(
                r#"
                    <div title="{}" class="frame" style="background-image: url('/img/{id}?key={}')"></div>
                "#,
                path.display(),
                args.key
            )
        })
        .collect::<String>();

    Ok(Html(format!(
        r#"
        <html>
            <head>
                <style>
                    :root {{
                        --frame-size: 15em;
                    }}

                    :root, html, body {{
                        margin: 0;
                        width: 100vw;
                        background-color: oklch(21.6% 0.006 56.043);
                    }}

                    main {{
                        margin: 2em;
                        width: calc(100% - 4em);
                        display: flex;
                        flex-wrap: wrap;
                        justify-content: space-around;
                        gap: 16px;
                    }}

                    .frame {{
                        width: calc(var(--frame-size) * 0.6);
                        height: var(--frame-size);

                        box-shadow: 10px 10px 15px 0px rgba(0,0,0,0.75);
                        
                        border: solid 1em oklch(86.8% 0.007 39.5);
                        border-radius: 6px;

                        background-position: center;
                        background-size: cover;
                        background-repeat: no-repeat;
                    }}

                    .frame.current {{
                        margin: 2em auto;
                    }}

                    hr {{
                        opacity: 15%;
                        margin-top: 4em;
                        margin-bottom: 4em;
                        margin-left: 2em;
                        margin-right: 2em;
                    }}
                </style>
            </head>
            <body>
                {current_element}
                <main>
                    {image_elements}
                </main>
            </body>
        </html>
        "#,
    )))
}

async fn serve_page(
    State(args): State<Args>,
    Query(query): Query<QueryArgs>,
) -> Result<Html<String>, StatusCode> {
    check_key(&args, &query)?;

    Ok(Html(format!(
        r#"
        <html>
            <head>
                <style>
                    :root, html, body {{
                        margin: 0;
                        width: 100vw;
                        height: 100vh;
                    }}

                    body {{
                        background-image: url("/img?key={}");
                        background-position: center;
                        background-size: cover;
                        background-repeat: no-repeat;
                    }}
                </style>
            </head>
            <body>
            </body>
        </html>
        "#,
        args.key
    )))
}

async fn serve_image(
    State(args): State<Args>,
    Query(query): Query<QueryArgs>,
) -> Result<Vec<u8>, StatusCode> {
    check_key(&args, &query)?;

    let picture_path = args.current_image_path();
    dbg!(&picture_path);

    let picture_data = tokio::fs::read(picture_path).await.unwrap();

    Ok(picture_data)
}

async fn serve_image_by_id(
    Path(id): Path<usize>,
    State(args): State<Args>,
    Query(query): Query<QueryArgs>,
) -> Result<Vec<u8>, StatusCode> {
    check_key(&args, &query)?;

    args.image_paths()
        .into_iter()
        .nth(id)
        .map(|path| std::fs::read(path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR))
        .ok_or(StatusCode::NOT_FOUND)
        .flatten()
}

impl Args {
    fn current_image_path(&self) -> PathBuf {
        let today = Zoned::now().date();
        // let seed = ((today.day_of_year() as u64) << 16) + (today.year() as u64);
        let seed = Zoned::now().time().second() as u64;
        let mut rng = StdRng::seed_from_u64(seed);

        self.image_paths().choose(&mut rng).unwrap().clone()
    }

    fn image_paths(&self) -> Vec<PathBuf> {
        let mut entries = std::fs::read_dir(&self.pictures)
            .unwrap()
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()
            .unwrap();

        entries.sort();

        entries.retain(|path| {
            let extension = path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();

            ["jpeg", "jpg", "tiff", "png", "heic"].contains(&extension.as_str())
        });

        entries
    }
}

fn check_key(args: &Args, query: &QueryArgs) -> Result<(), StatusCode> {
    if args.key != query.key {
        return Err(StatusCode::NOT_FOUND);
    } else {
        Ok(())
    }
}
