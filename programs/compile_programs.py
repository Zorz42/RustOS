import os

for program_dir in os.listdir("."):
    if os.path.isdir(program_dir):
        if os.path.isfile(program_dir + "/Cargo.toml"):
            print("Compiling " + program_dir)
            os.system("cd " + program_dir + " && cargo build --release")