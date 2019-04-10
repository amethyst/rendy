use {
    colorful::{core::color_string::CString, Color, Colorful as _},
    gfx_hal::memory::Properties,
};

/// Memory utilization stats.
#[derive(Clone, Copy, Debug)]
pub struct MemoryUtilization {
    /// Total number of bytes allocated.
    pub used: u64,
    /// Effective number bytes allocated.
    pub effective: u64,
}

/// Memory utilization of one heap.
#[derive(Clone, Copy, Debug)]
pub struct MemoryHeapUtilization {
    /// Utilization.
    pub utilization: MemoryUtilization,

    /// Memory heap size.
    pub size: u64,
}

/// Memory utilization of one type.
#[derive(Clone, Copy, Debug)]
pub struct MemoryTypeUtilization {
    /// Utilization.
    pub utilization: MemoryUtilization,

    /// Memory type info.
    pub properties: Properties,

    /// Index of heap this memory type uses.
    pub heap_index: usize,
}

/// Total memory utilization.
#[derive(Clone, Debug)]
pub struct TotalMemoryUtilization {
    /// Utilization by types.
    pub types: Vec<MemoryTypeUtilization>,

    /// Utilization by heaps.
    pub heaps: Vec<MemoryHeapUtilization>,
}

impl std::fmt::Display for TotalMemoryUtilization {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const MB: u64 = 1024 * 1024;

        writeln!(fmt, "!!! Memory utilization !!!")?;
        for (index, heap) in self.heaps.iter().enumerate() {
            let size = heap.size;
            let MemoryUtilization { used, effective } = heap.utilization;
            let permyriad = used * 10000 / size;
            let fill = if permyriad > 10000 {
                50
            } else {
                (permyriad / 200) as usize
            };
            let effective = if used > 0 {
                effective * 10000 / used
            } else {
                10000
            };

            let line = ("|".repeat(fill) + &(" ".repeat(50 - fill)))
                .gradient_with_color(Color::Green, Color::Red);
            writeln!(
                fmt,
                "Heap {}:\n{:6} / {:<6} or{} {{ effective:{} }} [{}]",
                format!("{}", index).magenta(),
                format!("{}MB", used / MB),
                format!("{}MB", size / MB),
                format_permyriad(permyriad),
                format_permyriad_inverted(effective),
                line
            )?;

            for ty in self.types.iter().filter(|ty| ty.heap_index == index) {
                let properties = ty.properties;
                let MemoryUtilization { used, effective } = ty.utilization;
                let permyriad = used * 10000 / size;
                let effective = if used > 0 {
                    effective * 10000 / used
                } else {
                    0
                };

                writeln!(
                    fmt,
                    "         {:>6} or{} {{ effective:{} }} | {:?}",
                    format!("{}MB", used / MB),
                    format_permyriad(permyriad),
                    format_permyriad_inverted(effective),
                    properties,
                )?;
            }
        }

        Ok(())
    }
}

fn format_permyriad(permyriad: u64) -> CString {
    debug_assert!(permyriad <= 10000);
    let s = format!("{:>3}.{:02}%", permyriad / 100, permyriad % 100);
    if permyriad > 7500 {
        s.red()
    } else if permyriad > 5000 {
        s.yellow()
    } else if permyriad > 2500 {
        s.green()
    } else if permyriad > 100 {
        s.blue()
    } else {
        s.white()
    }
}

fn format_permyriad_inverted(permyriad: u64) -> CString {
    debug_assert!(permyriad <= 10000);
    let s = format!("{:>3}.{:02}%", permyriad / 100, permyriad % 100);
    if permyriad > 9900 {
        s.white()
    } else if permyriad > 7500 {
        s.blue()
    } else if permyriad > 5000 {
        s.green()
    } else if permyriad > 2500 {
        s.yellow()
    } else {
        s.red()
    }
}
