import os
import shutil

root_dir = "projects"

for directory in os.listdir(root_dir):
    dir_path = os.path.join(root_dir, directory)
    if os.path.isdir(dir_path):
        os.system(f"cd {dir_path} && RUSTFLAGS=\"-C link-arg=-Tprogram_link.ld\" cargo build --release --target x86_64-unknown-none")
        shutil.copy(f"target/x86_64-unknown-none/release/{directory}", f"compiled_projects/{directory}")



