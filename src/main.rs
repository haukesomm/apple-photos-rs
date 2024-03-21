use std::path::Path;

use clap::{Args, Parser, Subcommand};

use crate::album_list::printer::AlbumListPrinter;
use crate::export::copying::{AssetCopyStrategy, DefaultAssetCopyStrategy, DryRunAssetCopyStrategy};
use crate::export::exporter::Exporter;
use crate::export::structure::{AlbumOutputStructureStrategy, JoiningOutputStructureStrategy, OutputStructureStrategy, PlainOutputStructureStrategy, YearMonthOutputStructureStrategy};
use crate::model::library::PhotosLibrary;
use crate::repo::album::AlbumRepository;
use crate::repo::asset::AssetRepository;
use crate::repo::asset::combining::CombiningAssetRepository;
use crate::repo::asset::default::{DefaultAssetWithAlbumInfoRepo, FilterMode};
use crate::repo::asset::hidden::HiddenAssetRepository;

mod model;
mod album_list;
mod repo;
mod export;
mod util;


/// Export photos from the macOS Photos library, organized by album and/or date.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Arguments {

    /// Path of the library file
    library_path: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {

    /// Lists all albums in the library
    ListAlbums,

    /// Exports the specified assets from the library to a given location
    ExportAssets(ExportArgs)
}

#[derive(Args, Debug)]
struct ExportArgs {

    /// Output directory
    output_dir: String,

    /// Output by album
    #[arg(short = 'a', long = "by-album", group = "strategy")]
    album: bool,

    /// Output by year/month
    #[arg(short = 'm', long = "by-year-month", group = "strategy")]
    year_month: bool,

    /// Output by year/month/album
    #[arg(short = 'M', long = "by-year-month-album", group = "strategy")]
    year_month_album: bool,

    /// Include albums matching the given ids
    #[arg(short = 'i', long = "include", group = "ids", num_args = 0.., value_delimiter = ' ')]
    include: Option<Vec<i32>>,

    /// Exclude albums matching the given ids
    #[arg(short = 'e', long = "exclude", group = "ids", num_args = 1.., value_delimiter = ' ')]
    exclude: Option<Vec<i32>>,

    /// Include hidden assets
    #[arg(short = 'H', long = "include-hidden")]
    include_hidden: bool,

    /// Restore original filenames
    #[arg(short = 'r', long = "restore-original-filenames")]
    restore_original_filenames: bool,

    /// Flatten album structure
    #[arg(short = 'f', long = "flatten-albums")]
    flatten_albums: bool,

    /// Dry run
    #[arg(short = 'd', long = "dry-run")]
    dry_run: bool,
}


fn main() {
    let args = Arguments::parse();

    let library = PhotosLibrary::new(args.library_path);

    match args.command {
        Commands::ListAlbums => list_albums(library.db_path()),
        Commands::ExportAssets(export_args) => export_assets(library, export_args)
    }
}


fn list_albums(db_path: String) {
    let album_lister = AlbumListPrinter::new(
        AlbumRepository::new(db_path)
    );
    album_lister.print_album_tree();
}


fn export_assets(photos_library: PhotosLibrary, args: ExportArgs) {
    let asset_repo = setup_asset_repo(photos_library.db_path(), &args);
    let output_strategy = setup_output_strategy(&args);
    let copy_strategy = setup_copy_strategy(&args);

    let exporter = Exporter::new(asset_repo, output_strategy, copy_strategy);
    exporter.export(
        Path::new(&photos_library.original_assets_path()),
        Path::new(&args.output_dir),
        args.restore_original_filenames
    );
}

fn setup_asset_repo(db_path: String, args: &ExportArgs) -> Box<dyn AssetRepository> {
    let mut asset_repos: Vec<Box<dyn AssetRepository>> = vec![
        {
            let filter = if let Some(ids) = args.include.clone() {
                FilterMode::IncludeAlbumIds(ids)
            } else if let Some(ids) = args.exclude.clone() {
                FilterMode::ExcludeAlbumIds(ids)
            } else {
                FilterMode::None
            };

            Box::new(DefaultAssetWithAlbumInfoRepo::new(db_path.clone(), filter))
        }
    ];

    if args.include_hidden {
        asset_repos.push(
            Box::new(HiddenAssetRepository::new(db_path.clone()))
        );
    }

    Box::new(
        CombiningAssetRepository::new(asset_repos)
    )
}

fn setup_output_strategy(args: &ExportArgs) -> Box<dyn OutputStructureStrategy> {
    if args.album {
        Box::new(AlbumOutputStructureStrategy::new(args.flatten_albums))
    } else if args.year_month {
        Box::new(YearMonthOutputStructureStrategy::asset_date_based())
    } else if args.year_month_album {
        Box::new(
            JoiningOutputStructureStrategy::new(
                vec![
                    Box::new(YearMonthOutputStructureStrategy::album_date_based()),
                    Box::new(AlbumOutputStructureStrategy::new(args.flatten_albums))
                ]
            )
        )
    } else {
        Box::new(PlainOutputStructureStrategy::new())
    }
}

fn setup_copy_strategy(args: &ExportArgs) -> Box<dyn AssetCopyStrategy> {
    if args.dry_run {
        Box::new(DryRunAssetCopyStrategy::new())
    } else {
        Box::new(DefaultAssetCopyStrategy::new())
    }
}