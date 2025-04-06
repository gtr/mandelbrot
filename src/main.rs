use core::str;
use image::{ImageBuffer, Rgb};
use num_complex::Complex64;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

const WIDTH: u32 = 2000;
const HEIGHT: u32 = 2000;
const MAX_ITERATIONS: u32 = 1000;
const MAX_ATTEMPTS: u32 = 400; // anything below 100 is not great lol

enum ColorScheme {
    Blue,
    Red,
    Rainbow,
    Greyscale,
    Blueish,
    Forest,
    Electric,
    Pastel,
    Monochrome(f64),
}

fn main() {
    fs::create_dir_all("output").expect("Failed to create output directory");
    let mut rng = rand::thread_rng();
    let color_scheme = get_random_color_scheme(&mut rng);

    let mut attempts = 0;
    let (mut center_re, mut center_im, mut zoom);

    loop {
        let (re, im, z) = find_interesting_region(&mut rng);
        if attempts >= MAX_ATTEMPTS || is_interesting_region(re, im, z) {
            center_re = re;
            center_im = im;
            zoom = z;
            break;
        }
        attempts += 1;
    }

    // first-person logs: https://twitter.com/mycoliza/status/1908609218334384196
    println!(
        "(at attempt #{}): I am generating a mandelbrot image for coordinate ({}, {}), zoom {}",
        attempts, center_re, center_im, zoom
    );
    let mandelbrot = generate_mandelbrot_img(center_re, center_im, zoom, &color_scheme);
    let scheme_name = get_color_scheme_name(&color_scheme);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();
    let filename = format!(
        "output/mandelbrot_{}_{}_{}_{}_{}.png",
        center_re, center_im, zoom, scheme_name, timestamp
    );
    mandelbrot.save(&filename).expect("Failed to save image");
    println!(
        "{} + {}i at zoom {:.10e}. ({})",
        center_re, center_im, zoom, scheme_name
    );
    println!("I saved the image as: {}", filename);
}

fn get_random_color_scheme(rng: &mut impl rand::Rng) -> ColorScheme {
    match rng.random_range(0..=8) {
        0 => ColorScheme::Blue,
        1 => ColorScheme::Red,
        2 => ColorScheme::Rainbow,
        3 => ColorScheme::Greyscale,
        4 => ColorScheme::Blueish,
        5 => ColorScheme::Forest,
        6 => ColorScheme::Electric,
        7 => ColorScheme::Pastel,
        _ => ColorScheme::Monochrome(rng.random::<f64>() * 360.0), // Random hue
    }
}

fn get_color_scheme_name(color_scheme: &ColorScheme) -> &str {
    match color_scheme {
        ColorScheme::Blue => "blues",
        ColorScheme::Red => "fire",
        ColorScheme::Rainbow => "rainbow",
        ColorScheme::Greyscale => "greyscale",
        ColorScheme::Blueish => "ocean",
        ColorScheme::Forest => "forest",
        ColorScheme::Electric => "electric",
        ColorScheme::Pastel => "pastel",
        ColorScheme::Monochrome(_) => "monochrome",
    }
}

fn find_interesting_region(rng: &mut impl rand::Rng) -> (f64, f64, f64) {
    let interesting_regions = [
        // (re, im, radius), weight
        ((-0.75, 0.1, 0.1), 0.2),    // main bulb boundary
        ((-0.16, 1.0, 0.05), 0.1),   // satellite bulb
        ((-0.77, 0.08, 0.2), 0.15),  // valley between large bulbs
        ((-1.25, 0.0, 0.2), 0.1),    // filaments
        ((-1.75, 0.0, 0.05), 0.05),  // period-3 bulb
        ((-0.9, 0.27, 0.13), 0.1),   // spiral formulation
        ((-0.12, 0.74, 0.02), 0.05), // detailed mini spirals
        ((0.2, 0.56, 0.02), 0.1),    // mini-Mandelbrot near boundary
        ((-1.4, 0.0, 0.1), 0.05),    // detailed edges
        ((-0.5, 0.56, 0.05), 0.1),   // dendrite formation
    ];

    let total_weight: f64 = interesting_regions.iter().map(|(_, w)| w).sum();
    let mut choice = rng.random::<f64>() * total_weight;

    for ((center_re, center_im, radius), weight) in interesting_regions.iter() {
        if choice <= *weight {
            let angle = rng.random::<f64>() * 2.0 * std::f64::consts::PI;
            let distance = rng.random::<f64>() * radius;
            let re = center_re + distance * angle.cos();
            let im = center_im + distance * angle.sin();
            let zoom_factor = if *radius < 0.05 { 100_000.0 } else { 10_000.0 };
            let zoom_base = zoom_factor / radius;
            let zoom = zoom_base * rng.random_range(0.1..=10.0);

            return (re, im, zoom);
        }
        choice -= weight;
    }
    // shouldn't reach here, but fallback to satisfy the compiler:
    (-0.75, 0.1, rng.random_range(1_000.0..=100_000.0))
}

// function to quickly test if a region is "interesting"
fn is_interesting_region(center_re: f64, center_im: f64, zoom: f64) -> bool {
    let scale = 4.0 / zoom;
    let re_min = center_re - scale / 2.0;
    let im_min = center_im - scale / 2.0;
    let mut iterations_histogram = [0; 5];
    let test_size = 30;

    for x in 0..test_size {
        for y in 0..test_size {
            let c = Complex64::new(
                re_min + (x as f64 / test_size as f64) * scale,
                im_min + (y as f64 / test_size as f64) * scale,
            );

            let iterations = mandelbrot_iterations(c);
            let bucket = if iterations == MAX_ITERATIONS {
                0
            } else {
                1 + (iterations * 4 / MAX_ITERATIONS) as usize
            };

            iterations_histogram[bucket] += 1;
        }
    }

    let inside_count = iterations_histogram[0];
    let total = test_size * test_size;

    // we want 5-95% of points to be inside the set
    let inside_ratio = inside_count as f64 / total as f64;
    if inside_ratio < 0.05 || inside_ratio > 0.95 {
        return false;
    }

    let non_zero_buckets = iterations_histogram[1..]
        .iter()
        .filter(|&&count| count > 0)
        .count();

    non_zero_buckets >= 1
}

fn generate_mandelbrot_img(
    center_re: f64,
    center_im: f64,
    zoom: f64,
    scheme: &ColorScheme,
) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(WIDTH, HEIGHT);

    let scale = 4.0 / zoom;
    let re_min = center_re - scale / 2.0;
    let im_min = center_im - scale / 2.0;

    for (x, y, pixel) in img.enumerate_pixels_mut() {
        // map pixel coordinates to complex plane
        let c = Complex64::new(
            re_min + (x as f64 / WIDTH as f64) * scale,
            im_min + (y as f64 / HEIGHT as f64) * scale,
        );

        // the main "coloring" algorithm: a simple implemetation of the "escape time" algorithm
        // more info: https://en.wikipedia.org/wiki/Plotting_algorithms_for_the_Mandelbrot_set
        let iterations = mandelbrot_iterations(c);
        *pixel = iterations_to_color(iterations, MAX_ITERATIONS, scheme);
    }

    img
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = match h_prime as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        5 => (c, 0.0, x),
        _ => (0.0, 0.0, 0.0),
    };

    let r = ((r1 + m) * 255.0) as u8;
    let g = ((g1 + m) * 255.0) as u8;
    let b = ((b1 + m) * 255.0) as u8;

    (r, g, b)
}

fn mandelbrot_iterations(c: Complex64) -> u32 {
    let mut z = Complex64::new(0.0, 0.0);
    let mut iterations = 0;

    //  we're working with complex numbers in the Mandelbrot set, not just real numbers so a^2 +
    //  b^2 <= 4
    while iterations < MAX_ITERATIONS && z.norm_sqr() <= 4.0 {
        // z = z² + c: determine how quickly this coordinate escapes
        z = z * z + c;
        iterations += 1;
    }

    iterations
}

fn iterations_to_color(
    iterations: u32,
    max_iterations: u32,
    color_scheme: &ColorScheme,
) -> Rgb<u8> {
    // these are the black areas -- the points that never escape
    if iterations == max_iterations {
        return Rgb([0, 0, 0]);
    }

    let speed = iterations as f64 / max_iterations as f64;
    match color_scheme {
        ColorScheme::Blue => {
            let hue = 240.0 - 60.0 * speed;
            let sat = 0.8 + 0.2 * speed;
            let val = 0.7 + 0.3 * speed;
            let (r, g, b) = hsv_to_rgb(hue, sat, val);
            Rgb([r, g, b])
        }
        ColorScheme::Red => {
            let hue = 60.0 * speed;
            let sat = 1.0;
            let val = 0.5 + 0.5 * speed;
            let (r, g, b) = hsv_to_rgb(hue, sat, val);
            Rgb([r, g, b])
        }
        ColorScheme::Rainbow => {
            let hue = 360.0 * speed;
            let sat = 0.8;
            let val = 0.9;
            let (r, g, b) = hsv_to_rgb(hue, sat, val);
            Rgb([r, g, b])
        }
        ColorScheme::Greyscale => {
            let val = (speed * 255.0) as u8;
            Rgb([val, val, val])
        }
        ColorScheme::Blueish => {
            let hue = 180.0 + 60.0 * speed;
            let sat = 0.7;
            let val = 0.5 + 0.5 * speed;
            let (r, g, b) = hsv_to_rgb(hue, sat, val);
            Rgb([r, g, b])
        }
        ColorScheme::Forest => {
            let hue = 120.0 - 40.0 * speed;
            let sat = 0.8 - 0.3 * speed;
            let val = 0.4 + 0.6 * speed;
            let (r, g, b) = hsv_to_rgb(hue, sat, val);
            Rgb([r, g, b])
        }
        ColorScheme::Electric => {
            let r = ((std::f64::consts::PI * speed * 8.0).sin() * 0.5 + 0.5) * 255.0;
            let g = ((std::f64::consts::PI * speed * 4.0).sin() * 0.5 + 0.5) * 255.0;
            let b = ((std::f64::consts::PI * speed * 2.0).sin() * 0.5 + 0.5) * 255.0;
            Rgb([r as u8, g as u8, b as u8])
        }
        ColorScheme::Pastel => {
            let hue = 360.0 * speed;
            let sat = 0.4;
            let val = 0.9;
            let (r, g, b) = hsv_to_rgb(hue, sat, val);
            Rgb([r, g, b])
        }
        ColorScheme::Monochrome(hue) => {
            let sat = 0.8;
            let val = speed;
            let (r, g, b) = hsv_to_rgb(*hue, sat, val);
            Rgb([r, g, b])
        }
    }
}
