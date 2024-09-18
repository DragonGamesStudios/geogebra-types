use geogebra_types::raw::{AngleUnit, Geogebra, Kernel, Val};

fn main() {
    let ggb = Geogebra {
        kernel: Kernel {
            digits: Val { val: 1 },
            angle_unit: Val {
                val: AngleUnit::Degree,
            },
            coord_style: Val { val: 1 },
        },
    };

    println!("{}", quick_xml::se::to_string(&ggb).unwrap());
}
