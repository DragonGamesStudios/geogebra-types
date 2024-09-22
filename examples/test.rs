use std::fs::File;

use geogebra_types::prelude::*;

fn main() {
    let mut ggb = Geogebra::new();

    let x = Numeric::complex(1.0, 2.0);
    let y = Numeric::complex(2.0, 3.0);
    ggb.var([x, y].sum());

    let out = File::create("out.ggb").unwrap();
    ggb.write(out).unwrap();
}
