use std::Vec;
use crate::ports::{byte_in, byte_out};
use crate::println;
use crate::timer::get_ticks;

const ATA_DATA: u16 = 0;
const ATA_ERROR: u16 = 1;
const ATA_SECTORCOUNT: u16 = 2;
const ATA_SECTORNUMBER1: u16 = 3;
const ATA_SECTORNUMBER2: u16 = 4;
const ATA_SECTORNUMBER3: u16 = 5;
const ATA_DRIVEHEAD: u16 = 6;
const ATA_STATUS: u16 = 7;
const ATA_COMMAND: u16 = 8;

#[derive(Debug)]
pub struct Disk {
    base: u16,
    h: u8,
    size: usize,
}

pub fn get_disk_status(base: u16) -> u8 {
    let res = byte_in(base | ATA_STATUS);
    if (res & 1) == 1 {
        let err = byte_in(base | ATA_ERROR);
        println!("Disk error: 0b{:b}", err);
    }
    res
}

pub fn scan_for_disks() -> Vec<Disk> {
    const BASES: [u16; 8] = [0x1F0, 0x3F0, 0x170, 0x370, 0x1E8, 0x3E0, 0x168, 0x360];

    let mut vec = Vec::new();

    for base in BASES {
        for h in 0..2 {
            byte_out(base | ATA_DRIVEHEAD, 0xA0 | (h << 4));

            if (get_disk_status(base) & 0b1110001) == 0b1010000 {
                byte_out(base | ATA_DRIVEHEAD, 0x40 | (h << 4));

                while (get_disk_status(base) & 0x40) == 0 {}

                byte_out(base | ATA_STATUS, 0xF8);

                println!("Scanning new disk");
                let started_waiting = get_ticks();
                let timed_out = loop {
                    if (get_disk_status(base) & 0b10000000) == 0 {
                        break false;
                    }

                    if get_ticks() > started_waiting + 10 {
                        break true;
                    }
                };
                if !timed_out {
                    let sectors_size =
                        ((byte_in(base + ATA_SECTORNUMBER1) as usize) << 0) +
                            ((byte_in(base + ATA_SECTORNUMBER2) as usize) << 8) +
                            ((byte_in(base + ATA_SECTORNUMBER3) as usize) << 16) +
                            ((byte_in(base + ATA_DRIVEHEAD) as usize) << 24) & 0xF;

                    vec.push(Disk {
                        base,
                        h,
                        size: sectors_size,
                    });
                }
            }
        }
    }

    vec
}