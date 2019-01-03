extern crate exoquant;
extern crate image;
extern crate imageproc;
extern crate rayon;

use exoquant::{convert_to_indexed, ditherer, optimizer, Color as ExoColor};
use image::Rgba;
use imageproc::drawing::draw_text_mut;
use rayon::prelude::*;
use rusttype::{FontCollection, Scale};
use std::{error::Error, fmt, fs, path::Path, process};

#[derive(Debug)]
struct Color {
    primary: [u8; 4],
    secondary: [u8; 4],
}

#[derive(Debug)]
struct Paths {
    first_path: String,
    second_path: String,
    export_path: String,
}

impl Paths {
    fn new(mut args: std::env::Args) -> Result<Paths, AppError> {
        if args.len() < 4 {
            return Err(AppError::NotEnoughArguments);
        }

        args.next();
        let first_path = args.next().unwrap();
        let second_path = args.next().unwrap();
        let export_path = args.next().unwrap();
        Ok(Paths {
            first_path,
            second_path,
            export_path,
        })
    }
}

#[derive(Debug)]
enum AppError {
    NotFound,
    MismatchSize,
    CouldntSaveFile,
    NotEnoughArguments,
}

impl Error for AppError {}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Oh no, something bad went down")
    }
}

impl From<image::ImageError> for AppError {
    fn from(_error: image::ImageError) -> Self {
        AppError::NotFound
    }
}

impl From<std::io::Error> for AppError {
    fn from(_error: std::io::Error) -> Self {
        AppError::CouldntSaveFile
    }
}

fn main() {
    let paths = match Paths::new(std::env::args()) {
        Ok(p) => p,
        Err(_) => {
            print_help();
            // close app.
            process::exit(1);
        }
    };

    let months: Vec<_> = fs::read_dir(&paths.first_path)
        .unwrap()
        .map(|res| res.unwrap().path())
        .collect();
    let images: Vec<_> = fs::read_dir(&paths.second_path)
        .unwrap()
        .map(|res| res.unwrap().path())
        .collect();

    images.par_iter().for_each(|image| {
        let color = get_color_palette(image.as_path());
        months.par_iter().for_each(|month| {
            // file_stem(), file_name() with extension
            let path = format!(
                "{}/{}-{}.png",
                &paths.export_path,
                month.file_stem().unwrap().to_str().unwrap(),
                image.file_stem().unwrap().to_str().unwrap()
            );
            let image_path = Path::new(&path);
            join_photos_vertically(image.as_path(), month.as_path(), image_path).unwrap();
            write_text(image_path, &color);
        });
    });
}

fn print_help() {
    println!("HELP:");
    println!("");
    println!("Simple programs, takes two folders, and combine the images in them, and put the results in the exported folder.");
    println!("");
    println!("USAGE:");
    println!("./program path1 path2 export_path");
    println!("================================================");
    println!("path1:         first folder path");
    println!("path2:         second folder path");
    println!("export_path:   folder where to save exported images.");
}

fn get_color_palette(path: &Path) -> Color {
    let img = image::open(path).expect("image couldn't be opened!");
    let img = img.to_rgba();
    let (width, _) = img.dimensions();
    let pixels: Vec<ExoColor> = img
        .into_raw()
        .chunks(4)
        .map(|c| ExoColor::new(c[0], c[1], c[2], c[3]))
        .collect();
    let (palette, _) = convert_to_indexed(
        &pixels,
        width as usize,
        256,
        &optimizer::KMeans,
        &ditherer::FloydSteinberg::new(),
    );
    // making sure always alpha is 255.
    let primary = [palette[0].r, palette[0].g, palette[0].b, 255u8];
    // random index, convert_to_index returns a Vec of len = 256
    let secondary = [palette[200].r, palette[200].g, palette[200].b, 255u8];
    Color { primary, secondary }
}

fn join_photos_vertically(
    first_path: &Path,
    second_path: &Path,
    result_path: &Path,
) -> Result<(), AppError> {
    let first_img = image::open(first_path)?;
    let second_img = image::open(second_path)?;

    let first_img = first_img.to_rgba();
    let second_img = second_img.to_rgba();
    let first_size = first_img.dimensions();
    let second_size = second_img.dimensions();

    // check if the width is not the same, kill it!
    if first_size.0 != second_size.0 {
        return Err(AppError::MismatchSize);
    }

    // getting the full width.
    let width = first_size.0;
    // joining up both heights
    let height = first_size.1 + second_size.1;

    let mut first_pxs = first_img.into_raw();
    let second_pxs = second_img.into_raw();

    first_pxs.extend(second_pxs);
    let buffer: &[u8] = &first_pxs; // Generate the image data

    // Save the buffer to result path.
    image::save_buffer(result_path, buffer, width, height, image::RGBA(8))?;
    Ok(())
}

fn write_text(path: &Path, color: &Color) -> u32 {
    // image path
    let path = Path::new(path);

    // create a new image buffer
    // let mut image = RgbImage::new(800, 800);
    let mut img = image::open(path).expect("File couldn't be opened!");

    // load the font as &[u8]
    let font = Vec::from(include_bytes!("JosefinSans-Thin.ttf") as &[u8]);

    //  load font.
    let font = FontCollection::from_bytes(font)
        .unwrap()
        .into_font()
        .unwrap();

    let height = 250.0;
    let scale = Scale {
        x: height * 1.0,
        y: height,
    };
    draw_text_mut(
        &mut img,
        Rgba(color.primary),
        380,
        580 + 2455,
        scale,
        &font,
        "20",
    );
    draw_text_mut(
        &mut img,
        Rgba(color.secondary),
        660,
        580 + 2455,
        scale,
        &font,
        "19",
    );

    match img.save(path) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}
