#![no_std]
#![no_main]

#[std::std_main]
fn main() {
    println!("Hello, world!");

    /*let mut curr_ticks = get_ticks() / 1000;
    loop {
        let ticks = get_ticks() + get_pid() * 0;

        if ticks / 1000 != curr_ticks {
            println!("Ticks {}: {}", get_pid(), get_ticks());
            curr_ticks = ticks / 1000;
        }
    }*/

    loop {
        sleep(1000);
        //println!("Ticks {}: {}", get_pid(), get_ticks());
    }
}