use image::open;
use img_hash::image::DynamicImage;
use img_hash::{FilterType, HasherConfig, ImageHash};
use rayon::prelude::*;
use std::fs::{self, create_dir_all};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const DIR_PATH: &str = "/home/cris/Documents/PERSONAL/";
const MAX_THREADS: usize = 10;
const MAX_DISTANCE: u32 = 10;
const TARGET_DIR: &str = "/home/cris/Documents/duplicados";

fn main() {
    let images = get_dir_images();
    println!("Found {} images", images.len());
    let image_hashes = process_images(images);
    let duplicated = find_similar_images(&image_hashes);
    println!("Found {:?} similar images", duplicated);
    move_duplicates(&duplicated);
}

fn process_images(images: Vec<PathBuf>) -> Vec<(ImageHash, PathBuf)> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(MAX_THREADS)
        .build()
        .unwrap();

    pool.install(|| {
        images
            .par_iter()
            .filter_map(|image_path| match get_image(image_path) {
                Some(image) => {
                    let hash = get_image_hash(&image);
                    println!("Image hash: {:?} for {:?}", hash, image_path);
                    Some((hash, image_path.clone()))
                }
                None => {
                    println!("Failed to process image: {:?}", image_path);
                    None
                }
            })
            .collect()
    })
}

fn get_dir_images() -> Vec<PathBuf> {
    let mut images = Vec::new();
    for entry in WalkDir::new(DIR_PATH) {
        let entry = entry.unwrap();
        if entry.file_type().is_file() {
            let path = entry.path();
            if is_image(path) {
                images.push(path.to_path_buf());
            }
        }
    }
    images
}

fn is_image(path: &Path) -> bool {
    let image_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "tiff"];
    if let Some(extension) = path.extension() {
        if let Some(extension_str) = extension.to_str() {
            return image_extensions.contains(&extension_str);
        }
    }
    false
}

fn get_image(path: &Path) -> Option<DynamicImage> {
    match open(path) {
        Ok(image) => {
            let processed = image.grayscale();
            Some(resize_image(&processed))
        }
        Err(e) => {
            eprintln!("Failed to open image: {:?}, error: {:?}", path, e);
            None
        }
    }
}

fn resize_image(image: &DynamicImage) -> DynamicImage {
    image.resize(64, 64, FilterType::Lanczos3)
}

fn get_image_hash(image: &DynamicImage) -> ImageHash {
    // Convertir la imagen a escala de grises (ya lo has hecho previamente)
    let gray_image = image.to_luma8(); // Ya tenemos la imagen en escala de grises

    // Usar la imagen en escala de grises para calcular el hash
    let hasher = HasherConfig::new().to_hasher();
    let hash = hasher.hash_image(&gray_image); // Aquí ya pasamos la imagen procesada

    hash
}

fn find_similar_images(
    image_hashes: &Vec<(ImageHash, PathBuf)>,
) -> Vec<((PathBuf, PathBuf), u32)> {
    let mut similar_images = Vec::new();

    for i in 0..image_hashes.len() {
        for j in (i + 1)..image_hashes.len() {
            let (hash1, path1) = &image_hashes[i];
            let (hash2, path2) = &image_hashes[j];
            let distance = hash1.dist(hash2);
            if distance <= MAX_DISTANCE {
                similar_images.push(((path1.clone(), path2.clone()), distance));
            }
        }
    }

    similar_images
}

fn move_duplicates(duplicates: &Vec<((PathBuf, PathBuf), u32)>) {
    // Crear carpeta de destino si no existe
    if let Err(e) = create_dir_all(TARGET_DIR) {
        eprintln!("Error creando carpeta de duplicados: {:?}", e);
        return;
    }

    for ((path1, path2), _) in duplicates {
        for original_path in &[path1, path2] {
            if let Some(file_name) = original_path.file_name() {
                let dest_path = Path::new(TARGET_DIR).join(file_name);

                // Si ya existe un archivo con el mismo nombre, añade un sufijo
                let mut final_path = dest_path.clone();
                let mut counter = 1;
                while final_path.exists() {
                    let new_file_name = format!(
                        "{}_{}.{}",
                        file_name
                            .to_string_lossy()
                            .rsplit_once('.')
                            .map(|(n, _)| n)
                            .unwrap_or("file"),
                        counter,
                        file_name
                            .to_string_lossy()
                            .rsplit_once('.')
                            .map(|(_, ext)| ext)
                            .unwrap_or("png")
                    );
                    final_path = Path::new(TARGET_DIR).join(new_file_name);
                    counter += 1;
                }

                match fs::copy(original_path, &final_path) {
                    Ok(_) => println!(
                        "Copiado: {} -> {}",
                        original_path.display(),
                        final_path.display()
                    ),
                    Err(e) => eprintln!("Error copiando {}: {:?}", original_path.display(), e),
                }
            }
        }
    }
}
