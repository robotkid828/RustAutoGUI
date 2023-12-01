use std::time::Duration;
use screenshots::Screen;
use enigo::{
  Enigo,
  MouseControllable
};
use image::{
  RgbImage,
  RgbaImage,
  Pixel
};
use image::io::Reader;

fn rgba_to_rgb(rgba_image: RgbaImage) -> RgbImage {
  //unoptimized, minimize use of this function
  let mut rgb_image = RgbImage::new(rgba_image.width(), rgba_image.height());
  let mut y = 0;
  for row in rgba_image.rows() {
    let mut x = 0;
    for rgba_pixel in row {
      rgb_image.put_pixel(x, y, rgba_pixel.to_rgb());
      x += 1;
    }
    y += 1;
  }
  return rgb_image
}

fn screenshot(screen: Screen, region_option: Option<Vec<u64>>, save_to_option: Option<String>) -> Result<RgbImage, String> {
  //pyautogui.screenshot()
  match save_to_option {
    Some(save_to) => {
      match region_option {
        Some(region) => {
          let captured = rgba_to_rgb(screen.capture_area(i32::try_from(region[0]).expect("Failed to convert number to i32."), i32::try_from(region[1]).expect("Failed to convert number to i32."), u32::try_from(region[2]).expect("Failed to convert number to i32."), u32::try_from(region[3]).expect("Failed to convert number to i32.")).expect("Failed to capture area."));
          let _ = captured.save(save_to);
          return Ok(captured)
        },
        None => {
          let captured = rgba_to_rgb(screen.capture().expect("Failed to capture screen."));
          let _ = captured.save(save_to);
          return Ok(captured)
        }
      }
    },
    None => {
      match region_option {
        Some(region) => {
          return Ok(rgba_to_rgb(screen.capture_area(region[0] as i32, region[1] as i32, region[2] as u32, region[3] as u32).expect("Failed to capture area.")))
        },
        None => {
          return Ok(rgba_to_rgb(screen.capture().expect("Failed to capture screen.")))
        }
      }
    }
  }
}

fn locate_on_screen(screen: Screen, needle_image: RgbImage, confidence_option: Option<f32>, steps_option: Option<u64>) -> Result<Vec<u64>, String> {
  //pyautogui.locateOnScreen()

  //method for locating on screen can be found at https://docs.opencv.org/3.4/de/da9/tutorial_template_matching.html
  //steps skips x - 1 pixels, this is for if performance is necessary. defaults to 1, NOT 0.
  let confidence: f32;
  match confidence_option {
    Some(matched_confidence) => confidence = matched_confidence,
    None => confidence = 0.999
  }
  let steps: u64;
  match steps_option {
    Some(0) => steps = 1,
    Some(matched_steps) => steps = matched_steps,
    None => steps = 1
  }
  let haystack_image = screenshot(screen, None, None).expect("Failed to take a screenshot.");

  //some error handling before we actually begin locating
  if confidence < 0.0  {
    return Err("Confidence is below 0. (Confidence must be a value between 0 and 1.)".to_string())
  }
  if confidence > 1.0 {
    return Err("Confidence is above 1. (Confidence must be a value between 0 and 1.)".to_string())
  }
  if needle_image.width() > haystack_image.width() {
    return Err("Needle image width is greater than haystack image width.".to_string())
  }
  if needle_image.height() > haystack_image.height() {
    return Err("Needle image height is greater than haystack image height.".to_string())
  }
  if steps > (haystack_image.width() * haystack_image.height()) as u64 {
    return Err("Steps number is greater than the dimensions of the screen. (Must be below (screen width * screen height).)".to_string())
  }

  let mut best_score: u64 = u64::MAX;
  let mut best_x: u64 = 0;
  let mut best_y: u64 = 0;
  for y in 0..haystack_image.height() - needle_image.height() {
    for x in 0..((haystack_image.width() - needle_image.width()) as f64 / steps as f64).round() as u64 {
      let mut scores = Vec::new();
      for reference_y in 0..needle_image.height() {
        for reference_x in 0..needle_image.width() {
          let haystack_pixel = haystack_image.get_pixel((x + reference_x as u64) as u32, y + reference_y);
          let needle_pixel = needle_image.get_pixel(reference_x, reference_y);
          scores.push(((((haystack_pixel[0] as i64 - needle_pixel[0] as i64).abs() as u64) + ((haystack_pixel[1] as i64 - needle_pixel[1] as i64).abs() as u64) + ((haystack_pixel[2] as i64 - needle_pixel[2] as i64).abs() as u64)) as f64 / 3.0) as u64);
        }
      }
      let mut overall_score: u64 = 0;
      for score in &scores {
        overall_score += score;
      }
      overall_score = (overall_score as f64 / scores.len() as f64).round() as u64;
      if overall_score < best_score {
        let mut enigo = Enigo::new();
        best_score = overall_score;
        best_x = x;
        best_y = y as u64;
      }
    }
  }
  if best_score != u64::MAX {
    //hashmap just feels unnecessary, and f64 feels odd when x and y are normally u64
    return Ok(vec![best_x, best_y/*, (255 - best_score) / 255*/])
  }
  return Err("Locate function failed to return.".to_string())
}

fn move_to(mut enigo: Enigo, og_x: u64, og_y: u64, x: u64, y: u64, time_option: Option<f64>) {
  let time: f64;
  match time_option {
    Some(matched_time) => time = matched_time,
    None => time = 0.0
  }
  if time == 0.0 {
    enigo.mouse_move_to(i32::try_from(x).expect("Failed to convert number to i32."), i32::try_from(y).expect("Failed to convert number to i32."));
  } else {
    let mut tweens: Vec<Vec<u64>> = Vec::new();
    let tween_increment = (y as i64 - og_y as i64) as f64 / (x as i64 - og_x as i64) as f64;
    let mut tween_amount = tween_increment.clone();
    if tween_increment > 0.0 {
      let mut y_offset: i64 = 0;
      let mut x_offset: i64 = 0;
      for _infinite_loop_break in 0..(enigo.main_display_size().0 * enigo.main_display_size().1) {
        while tween_amount >= 1.0 {
          let push_tween = vec![(i64::try_from(og_x).expect("Failed to convert number to i64.") + x_offset) as u64, (i64::try_from(og_y).expect("Failed to convert number to i64.") + y_offset) as u64];
          if !tweens.iter().any(|item| item == &push_tween) {
            tweens.push(push_tween);
          }
          tween_amount -= 1.0;
          if y > og_y {
            y_offset += 1;
          } else {
            y_offset -= 1;
          }
        }
        if x > og_x {
          x_offset += 1;
        } else {
          x_offset -= 1;
        }
        if x > og_x {
          if (og_x + x_offset as u64) >= x {
            break
          }
        } else if x < og_x {
          if (i64::try_from(og_x).expect("Failed to convert number to i64.") + x_offset) as u64 <= x {
            break
          }
        } else if y > og_y {
          if (og_y + y_offset as u64) >= y {
            break
          }
        } else if y < og_y {
          if (i64::try_from(og_y).expect("Failed to convert number to i64.") + y_offset) as u64 <= y {
            break
          }
        }
        let push_tween = vec![(i64::try_from(og_x).expect("Failed to convert number to i64.") + x_offset) as u64, (i64::try_from(og_y).expect("Failed to convert number to i64.") + y_offset) as u64];
        if !tweens.iter().any(|item| item == &push_tween) {
          tweens.push(push_tween);
        }
        tween_amount += tween_increment;
      }
    } else if tween_increment == 0.0 {
      let mut x_offset: i64 = 0;
      for _infinite_loop_break in 0..enigo.main_display_size().0 {
        let push_tween = vec![(i64::try_from(og_x).expect("Failed to convert number to i64.") + x_offset) as u64, y];
        if !tweens.iter().any(|item| item == &push_tween) {
          tweens.push(push_tween);
        }
        if x > og_x {
          x_offset += 1;
        } else {
          x_offset -= 1;
        }
        if x > og_x {
          if (og_x + x_offset as u64) >= x {
            break
          }
        } else if x < og_x {
          if (i64::try_from(og_x).expect("Failed to convert number to i64.") + x_offset) as u64 <= x {
            break
          }
        }
      }
    } else if tween_increment < 0.0 {
      let mut y_offset: i64 = 0;
      let mut x_offset: i64 = 0;
      for _infinite_loop_break in 0..(enigo.main_display_size().0 * enigo.main_display_size().1) {
        while tween_amount <= 1.0 {
          let push_tween = vec![(i64::try_from(og_x).expect("Failed to convert number to i64.") + x_offset) as u64, (i64::try_from(og_y).expect("Failed to convert number to i64.") + y_offset) as u64];
          if !tweens.iter().any(|item| item == &push_tween) {
            tweens.push(push_tween);
          }
          tween_amount += 1.0;
          if y > og_y {
            y_offset += 1;
          } else {
            y_offset -= 1;
          }
        }
        if x > og_x {
          x_offset += 1;
        } else {
          x_offset -= 1;
        }
        if x > og_x {
          if (og_x + x_offset as u64) >= x {
            break
          }
        } else if x < og_x {
          if (i64::try_from(og_x).expect("Failed to convert number to i64.") + x_offset) as u64 <= x {
            break
          }
        } else if y > og_y {
          if (og_y + y_offset as u64) >= y {
            break
          }
        } else if y < og_y {
          if (i64::try_from(og_y).expect("Failed to convert number to i64.") + y_offset) as u64 <= y {
            break
          }
        }
        let push_tween = vec![(i64::try_from(og_x).expect("Failed to convert number to i64.") + x_offset) as u64, (i64::try_from(og_y).expect("Failed to convert number to i64.") + y_offset) as u64];
        if !tweens.iter().any(|item| item == &push_tween) {
          tweens.push(push_tween);
        }
        tween_amount += tween_increment;
      }
    }
    tweens.push(vec![x, y]);
    let wait_duration = time / tweens.len() as f64;
    for tween in tweens {
      enigo.mouse_move_to(i32::try_from(tween[0]).expect("Failed to convert number to i32."), i32::try_from(tween[1]).expect("Failed to convert number to i32."));
      spin_sleep::sleep(Duration::from_nanos((wait_duration * 1000000000.0).round() as u64));
    }
  }
}

fn main() {
  let screen = Screen::all().unwrap()[0];
  let mut enigo = Enigo::new();
}