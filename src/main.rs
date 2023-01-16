use std::collections::VecDeque;
use std::env;
use std::fs;
use std::fs::ReadDir;
use std::os::unix::fs::symlink;
use std::path::Path;

fn main() {
	let mut args: VecDeque<String> = env::args().collect();
	args.pop_front();

	if args.len() < 2 {
		println!("Usage: dotz [path] [destination]");
		return;
	}

	let path = match Path::new(args.pop_front().unwrap().as_str()).canonicalize() {
		Ok(path) => path,
		Err(_) => {
			println!("Path must be a directory");
			println!("Usage: dotz [path] [destination]");
			return;
		}
	};

	let destination = match Path::new(args.pop_front().unwrap().as_str()).canonicalize() {
		Ok(path) => path,
		Err(_) => {
			println!("Path must be a directory");
			println!("Usage: dotz [path] [destination]");
			return;
		}
	};

	let files = match fs::read_dir(path) {
		Ok(files) => files,
		Err(_) => {
			println!("Path must be a directory");
			println!("Usage: dotz [path] [destination]");
			return;
		}
	};

	create_symlinks(files, &destination);
}

fn create_symlinks(files: ReadDir, destination: &Path) {
	for file in files {
		let file = file.unwrap();
		let file_path = file.path();

		if file.file_type().unwrap().is_dir() {
			let files = fs::read_dir(&file_path).unwrap();
			let dest = destination.clone().join(file_path.file_name().unwrap());

			match fs::create_dir(&dest) {
				Ok(_) => {
					println!("Created directory {}", dest.display());
				}
				Err(_) => {}
			};

			create_symlinks(files, &dest);
			return;
		}

		let dest = Path::new(&destination).join(file_path.file_name().unwrap());

		println!("Linking {} to {}", file_path.display(), dest.display());

		match symlink(&file_path, &dest) {
			Ok(_) => (),
			Err(e) => {
				println!(
					"Failed to link {} to {}: {}",
					file_path.display(),
					dest.display(),
					e
				);
				continue;
			}
		};
	}
}
