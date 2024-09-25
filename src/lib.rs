//! This is an attempt at creating a library for working with GeoGebra files.
//! This is largely an educated guess and what works how as the documentation
//! on the format is incredibly sparse. This is mostly incomplete and is mostly
//! meant as a utility crate for Geo-AID.

use std::{
    io::{self, Seek, Write},
    marker::PhantomData,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign},
    rc::Rc,
};

use num_traits::{Bounded, Num, One, Zero};
use raw::{
    Construction, ConstructionItem, Coords, Element, ElementType, LabelMode, ObjColorType, Show,
};
use zip::{write::FileOptions, ZipWriter};

pub mod raw;
pub use raw::{LineStyle, LineType};

pub mod prelude {
    pub use super::{
        Conic, ConicAccess, Expr as _, Geogebra, Line, List, ListAccess, Numeric, NumericAccess,
        Point, PointAccess, Ray, Segment,
    };
}

/// High-level API for working with a Geogebra workspace.
#[derive(Debug)]
pub struct Geogebra {
    data: raw::Geogebra,
    /// Next element id to use for a label
    next_id: usize,
}

impl Geogebra {
    /// Create a new, empty workspace.
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: raw::Geogebra {
                format: String::from("5.0"),
                construction: Construction::default(),
                app: String::from("suite"),
                sub_app: String::from("geometry"),
            },
            next_id: 0,
        }
    }

    /// Write the ggb file to a stream.
    pub fn write(&self, stream: impl Write + Seek) -> io::Result<()> {
        let geogebra = quick_xml::se::to_string(&self.data).unwrap();

        let mut file = ZipWriter::new(stream);

        file.start_file("geogebra.xml", FileOptions::<()>::default())?;
        file.write_all(b"<?xml version=\"1.0\" encoding=\"utf-8\" ?>")?;
        file.write_all(geogebra.as_bytes())?;
        file.finish()?;

        Ok(())
    }

    fn next_label(&mut self) -> String {
        let mut next_label = format!("elem{}", self.next_id);
        self.next_id += 1;

        while self.data.construction.items.iter().any(|item| match item {
            ConstructionItem::Element(element) => element.label == next_label,
            ConstructionItem::Command(_) => false,
            ConstructionItem::Expression(expression) => expression.label == next_label,
        }) {
            next_label = format!("elem{}", self.next_id);
            self.next_id += 1;
        }

        next_label
    }
}

#[derive(Clone, Copy)]
struct Style {
    /// Whether to display the point's label
    pub display_label: bool,
    /// Settings for line display.
    pub line_style: Option<LineStyle>,
    /// Color of this object
    pub color: Option<ObjColorType>,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            display_label: true,
            line_style: None,
            color: None,
        }
    }
}

impl Style {
    #[must_use]
    fn to_element(self) -> Element {
        Element {
            type_: ElementType::Point,
            label: String::new(),
            caption: None,
            label_mode: LabelMode::Caption.into(),
            show: Show {
                object: true,
                label: self.display_label,
            },
            coords: None,
            line_style: self.line_style,
            obj_color: self.color,
        }
    }
}

pub trait Object: Into<Expression> {}

/// An immutable labeled expression. Passed by reference
pub struct Var<T>(Rc<String>, PhantomData<T>);

impl<T> Var<T> {
    #[must_use]
    fn new(expr: String) -> Self {
        Self(Rc::new(expr), PhantomData)
    }
}

impl<T: Object> Object for Var<T> {}

impl<T: Object> Object for &Var<T> {}

impl<T: Expr> Expr for Var<T> {
    type Target = T::Target;

    fn get_type() -> ElementType {
        T::get_type()
    }

    fn var(expr: String) -> Var<Self::Target> {
        T::var(expr)
    }
}

impl<T: Expr> Expr for &Var<T> {
    type Target = T::Target;

    fn get_type() -> ElementType {
        T::get_type()
    }

    fn var(expr: String) -> Var<Self::Target> {
        T::var(expr)
    }
}

/// A type-erased expression.
#[derive(Clone)]
pub struct Expression {
    expr: Rc<String>,
    style: Style,
}

impl Expression {
    /// Expression from a string, default style.
    pub fn expr(expr: impl ToString) -> Self {
        Self {
            expr: Rc::new(expr.to_string()),
            style: Style::default(),
        }
    }
}

pub trait Expr: Into<Expression> {
    /// Target primitive type.
    type Target;

    /// Get the element type of this expression
    fn get_type() -> ElementType;

    /// Create a variable of target type
    #[must_use]
    fn var(expr: String) -> Var<Self::Target>;
}

impl<X: Into<Numeric>, Y: Into<Numeric>> From<(X, Y)> for Expression {
    fn from((x, y): (X, Y)) -> Self {
        Self {
            expr: Rc::new(format!(
                "(real({}), real({}))",
                x.into().0.expr,
                y.into().0.expr
            )),
            style: Style::default(),
        }
    }
}

impl From<f64> for Expression {
    fn from(value: f64) -> Self {
        Self {
            expr: Rc::new(format!("{value} + 0i")),
            style: Style::default(),
        }
    }
}

impl<T> From<Var<T>> for Expression {
    fn from(value: Var<T>) -> Self {
        Self::from(&value)
    }
}

impl<T> From<&Var<T>> for Expression {
    fn from(value: &Var<T>) -> Self {
        Self {
            expr: Rc::clone(&value.0),
            style: Style::default(),
        }
    }
}

impl From<Point> for Expression {
    fn from(value: Point) -> Self {
        value.0
    }
}

/// A point element of the Geogebra construction
#[derive(Clone)]
pub struct Point(Expression);

impl Point {
    /// Set the line's color
    pub fn set_color(&mut self, r: u8, g: u8, b: u8) {
        self.0.style.color = Some(ObjColorType { r, g, b });
    }

    /// Wether to display this line's label
    pub fn set_display_label(&mut self, v: bool) {
        self.0.style.display_label = v;
    }

    /// Style for a point bound to its expression
    #[must_use]
    fn bound() -> Style {
        Style {
            display_label: true,
            line_style: None,
            color: Some(ObjColorType {
                r: 97,
                g: 97,
                b: 97,
            }),
        }
    }

    /// Style for a point bound to its expression
    #[must_use]
    fn free() -> Style {
        Style {
            display_label: true,
            line_style: None,
            color: Some(ObjColorType {
                r: 21,
                g: 101,
                b: 192,
            }),
        }
    }

    /// Intersection of two lines
    #[must_use]
    pub fn intersect(k: impl Into<Line>, l: impl Into<Line>) -> Self {
        Self(Expression {
            expr: Rc::new(format!(
                "Intersect({}, {})",
                k.into().0.expr,
                l.into().0.expr
            )),
            style: Self::bound(),
        })
    }

    /// Point on another geometric object
    #[must_use]
    pub fn on(v: impl Object) -> Self {
        Self(Expression {
            expr: Rc::new(format!("Point({})", v.into().expr)),
            style: Self::free(),
        })
    }

    /// Get the x coordinate of this point
    #[must_use]
    pub fn x(self) -> Numeric {
        Numeric(Expression {
            expr: Rc::new(format!("x({})", self.0.expr)),
            style: Style::default(),
        })
    }

    /// Get the y coordinate of this point
    #[must_use]
    pub fn y(self) -> Numeric {
        Numeric(Expression {
            expr: Rc::new(format!("y({})", self.0.expr)),
            style: Style::default(),
        })
    }

    /// Convert this point to a complex number
    #[must_use]
    pub fn complex(self) -> Numeric {
        Numeric(Expression::expr(format!("ToComplex({})", self.0.expr)))
    }
}

impl<X: Into<Numeric>, Y: Into<Numeric>> From<(X, Y)> for Point {
    fn from(value: (X, Y)) -> Self {
        Self(Expression::from(value))
    }
}

impl From<Var<Point>> for Point {
    fn from(value: Var<Point>) -> Self {
        Self(Expression::from(value))
    }
}

impl From<&Var<Point>> for Point {
    fn from(value: &Var<Point>) -> Self {
        Self(Expression::from(value))
    }
}

impl Expr for Point {
    type Target = Self;

    fn get_type() -> ElementType {
        ElementType::Point
    }

    fn var(expr: String) -> Var<Self::Target> {
        Var::new(expr)
    }
}

impl Object for Point {}

impl<X: Into<Numeric>, Y: Into<Numeric>> Expr for (X, Y) {
    type Target = Point;

    fn get_type() -> ElementType {
        ElementType::Point
    }

    fn var(expr: String) -> Var<Self::Target> {
        Var::new(expr)
    }
}

/// Trait with point-related functions
pub trait PointAccess: Sized
where
    Point: From<Self>,
{
    /// Get the x coordinate of this point
    #[must_use]
    fn x(self) -> Numeric {
        Point::from(self).x()
    }

    /// Get the y coordinate of this point
    #[must_use]
    fn y(self) -> Numeric {
        Point::from(self).x()
    }

    /// Convert to a complex number
    #[must_use]
    fn complex(self) -> Numeric {
        Point::from(self).complex()
    }
}

impl<T> PointAccess for T where Point: From<T> {}

impl Expr for f64 {
    type Target = Numeric;

    fn get_type() -> ElementType {
        ElementType::Numeric
    }

    fn var(expr: String) -> Var<Self::Target> {
        Var::new(expr)
    }
}

/// A line element of the Geogebra construction
#[derive(Clone)]
pub struct Line(Expression);

impl Line {
    /// Set the line's color
    pub fn set_color(&mut self, r: u8, g: u8, b: u8) {
        self.0.style.color = Some(ObjColorType { r, g, b });
    }

    /// Set the line's style
    pub fn set_style(&mut self, style: LineStyle) {
        self.0.style.line_style = Some(style);
    }

    /// Wether to display this line's label
    pub fn set_display_label(&mut self, v: bool) {
        self.0.style.display_label = v;
    }

    /// Default line style.
    #[must_use]
    fn style() -> Style {
        Style {
            display_label: false,
            line_style: Some(LineStyle::default()),
            color: None,
        }
    }

    /// Make a line through two points.
    #[must_use]
    pub fn new(a: impl Into<Point>, b: impl Into<Point>) -> Self {
        Self(Expression {
            expr: Rc::new(format!("Line({}, {})", a.into().0.expr, b.into().0.expr)),
            style: Self::style(),
        })
    }

    /// Bisector of an angle
    #[must_use]
    pub fn angle_bisector(a: impl Into<Point>, b: impl Into<Point>, c: impl Into<Point>) -> Self {
        Self(Expression::expr(format!(
            "AngleBisector({}, {}, {})",
            a.into().0.expr,
            b.into().0.expr,
            c.into().0.expr
        )))
    }

    /// A line perpendicular to another, going through a point
    #[must_use]
    pub fn perpendicular(to: impl Into<Line>, through: impl Into<Point>) -> Self {
        Self(Expression::expr(format!(
            "PerpendicularLine({}, {})",
            through.into().0.expr,
            to.into().0.expr
        )))
    }

    /// A line parallel to another, going through a point
    #[must_use]
    pub fn parallel(to: impl Into<Line>, through: impl Into<Point>) -> Self {
        Self(Expression::expr(format!(
            "Line({}, {})",
            through.into().0.expr,
            to.into().0.expr
        )))
    }
}

impl From<Var<Line>> for Line {
    fn from(value: Var<Line>) -> Self {
        Self(Expression::from(value))
    }
}

impl From<&Var<Line>> for Line {
    fn from(value: &Var<Line>) -> Self {
        Self(Expression::from(value))
    }
}

impl From<Line> for Expression {
    fn from(value: Line) -> Self {
        value.0
    }
}

impl Expr for Line {
    type Target = Self;

    fn get_type() -> ElementType {
        ElementType::Line
    }

    fn var(expr: String) -> Var<Self::Target> {
        Var::new(expr)
    }
}

impl Object for Line {}

/// A list expression
#[derive(Clone)]
pub struct List<T>(Expression, PhantomData<T>);

impl<T: Expr, It: IntoIterator<Item = T>> From<It> for List<T::Target>
where
    Expression: From<T>,
{
    fn from(value: It) -> Self {
        let mut args = String::new();

        for arg in value {
            args += Expression::from(arg).expr.as_ref();
            args += ", "
        }

        args.pop();
        args.pop();

        Self(
            Expression {
                expr: Rc::new(format!("{{{args}}}")),
                style: Style::default(),
            },
            PhantomData,
        )
    }
}

impl<T> From<List<T>> for Expression {
    fn from(value: List<T>) -> Self {
        value.0
    }
}

impl<T> Expr for List<T> {
    type Target = List<T>;

    fn get_type() -> ElementType {
        ElementType::List
    }

    fn var(expr: String) -> Var<Self::Target> {
        Var::new(expr)
    }
}

impl<T> From<&Var<List<T>>> for List<T> {
    fn from(value: &Var<List<T>>) -> Self {
        Self(value.into(), PhantomData)
    }
}

impl List<Point> {
    /// Mean value of X coordinates of points.
    #[must_use]
    pub fn mean_x(self) -> Numeric {
        Numeric(Expression {
            expr: Rc::new(format!("MeanX({})", self.0.expr)),
            style: Style::default(),
        })
    }

    /// Mean value of Y coordinates of points.
    #[must_use]
    pub fn mean_y(self) -> Numeric {
        Numeric(Expression {
            expr: Rc::new(format!("MeanX({})", self.0.expr)),
            style: Style::default(),
        })
    }
}

impl List<Numeric> {
    /// Sum of these numbers
    #[must_use]
    pub fn sum(self) -> Numeric {
        Numeric(Expression {
            expr: Rc::new(format!("Sum(Append({}, 0 + 0i))", self.0.expr)),
            style: Style::default(),
        })
    }

    /// Product of these numbers
    #[must_use]
    pub fn product(self) -> Numeric {
        Numeric(Expression {
            expr: Rc::new(format!("Product(Append({}, 1 + 0i))", self.0.expr)),
            style: Style::default(),
        })
    }
}

/// A trait for accessing list functions through convertible types
pub trait ListAccess<T>: Sized
where
    List<T>: From<Self>,
{
    /// Get the mean X coordinate
    fn mean_x(self) -> Numeric
    where
        List<Point>: From<Self>,
    {
        List::from(self).mean_x()
    }

    /// Get the mean Y coordinate
    fn mean_y(self) -> Numeric
    where
        List<Point>: From<Self>,
    {
        List::from(self).mean_y()
    }

    /// Get the sum of numbers
    fn sum(self) -> Numeric
    where
        List<Numeric>: From<Self>,
    {
        List::from(self).sum()
    }

    /// Get the product of numbers
    fn product(self) -> Numeric
    where
        List<Numeric>: From<Self>,
    {
        List::from(self).product()
    }
}

impl<T, V> ListAccess<T> for V where List<T>: From<V> {}

/// A number value
#[derive(Clone)]
pub struct Numeric(Expression);

impl Numeric {
    /// Check if this numeric is a constant
    #[must_use]
    pub fn is_const(&self) -> bool {
        self.0.expr.parse::<f64>().is_ok()
    }

    /// Distance between a point and an object
    #[must_use]
    pub fn distance<T: Object>(point: impl Into<Point>, object: T) -> Self {
        Self(Expression {
            expr: Rc::new(format!(
                "Distance({}, {})",
                point.into().0.expr,
                object.into().expr
            )),
            style: Style::default(),
        })
    }

    /// A complex number
    #[must_use]
    pub fn complex(real: impl Into<Numeric>, imaginary: impl Into<Numeric>) -> Self {
        Self(Expression {
            expr: Rc::new(format!(
                "({}) + ({})i",
                real.into().0.expr,
                imaginary.into().0.expr
            )),
            style: Style::default(),
        })
    }

    /// An angle defined by three points
    #[must_use]
    pub fn angle(a: impl Into<Point>, b: impl Into<Point>, c: impl Into<Point>) -> Self {
        Self(Expression::expr(format!(
            "Angle({}, {}, {})",
            a.into().0.expr,
            b.into().0.expr,
            c.into().0.expr
        )))
    }

    /// Angle between two lines
    #[must_use]
    pub fn angle_lines(k: impl Into<Line>, l: impl Into<Line>) -> Self {
        Self(Expression::expr(format!(
            "Angle({}, {})",
            k.into().0.expr,
            l.into().0.expr
        )))
    }

    /// atan2 function
    #[must_use]
    pub fn atan2(y: impl Into<Numeric>, x: impl Into<Numeric>) -> Self {
        Self(Expression::expr(format!(
            "atan2({}, {})",
            y.into().0.expr,
            x.into().0.expr
        )))
    }

    /// Raise this number to a power.
    #[must_use]
    pub fn pow(self, exponent: impl Into<Numeric>) -> Self {
        Self(Expression::expr(format!(
            "({})^({})",
            self.0.expr,
            exponent.into().0.expr
        )))
    }

    /// Get the real part of this number
    #[must_use]
    pub fn real(self) -> Self {
        Self(Expression::expr(format!("real({})", self.0.expr)))
    }

    /// Get the imaginary part of this number
    #[must_use]
    pub fn imaginary(self) -> Self {
        Self(Expression::expr(format!("imaginary({})", self.0.expr)))
    }

    /// Get the argument of a complex number.
    #[must_use]
    pub fn arg(self) -> Self {
        Self(Expression::expr(format!("arg({})", self.0.expr)))
    }
}

impl From<f64> for Numeric {
    fn from(value: f64) -> Self {
        Self(Expression::from(value))
    }
}

impl From<&Var<Numeric>> for Numeric {
    fn from(value: &Var<Numeric>) -> Self {
        Self(Expression::from(value))
    }
}

impl From<Numeric> for Expression {
    fn from(value: Numeric) -> Self {
        value.0
    }
}

impl Expr for Numeric {
    type Target = Self;

    fn get_type() -> ElementType {
        ElementType::Numeric
    }

    fn var(expr: String) -> Var<Self::Target> {
        Var::new(expr)
    }
}

impl<T: Into<Numeric>> Add<T> for Numeric {
    type Output = Self;

    fn add(mut self, rhs: T) -> Self::Output {
        self += rhs;
        self
    }
}

impl<T: Into<Numeric>> AddAssign<T> for Numeric {
    fn add_assign(&mut self, rhs: T) {
        let expr = Expression {
            expr: Rc::new(format!("({}) + ({})", self.0.expr, rhs.into().0.expr)),
            style: Style::default(),
        };
        self.0 = expr;
    }
}

impl<T: Into<Numeric>> Sub<T> for Numeric {
    type Output = Self;

    fn sub(mut self, rhs: T) -> Self::Output {
        self -= rhs;
        self
    }
}

impl<T: Into<Numeric>> SubAssign<T> for Numeric {
    fn sub_assign(&mut self, rhs: T) {
        let expr = Expression {
            expr: Rc::new(format!("({}) - ({})", self.0.expr, rhs.into().0.expr)),
            style: Style::default(),
        };
        self.0 = expr;
    }
}

impl<T: Into<Numeric>> Mul<T> for Numeric {
    type Output = Self;

    fn mul(mut self, rhs: T) -> Self::Output {
        self *= rhs;
        self
    }
}

impl<T: Into<Numeric>> MulAssign<T> for Numeric {
    fn mul_assign(&mut self, rhs: T) {
        let expr = Expression {
            expr: Rc::new(format!("({}) * ({})", self.0.expr, rhs.into().0.expr)),
            style: Style::default(),
        };
        self.0 = expr;
    }
}

impl<T: Into<Numeric>> Div<T> for Numeric {
    type Output = Self;

    fn div(mut self, rhs: T) -> Self::Output {
        self /= rhs;
        self
    }
}

impl<T: Into<Numeric>> DivAssign<T> for Numeric {
    fn div_assign(&mut self, rhs: T) {
        let expr = Expression {
            expr: Rc::new(format!("({}) / ({})", self.0.expr, rhs.into().0.expr)),
            style: Style::default(),
        };
        self.0 = expr;
    }
}

impl Neg for Numeric {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(Expression {
            expr: Rc::new(format!("-({})", self.0.expr)),
            style: Style::default(),
        })
    }
}

impl<T: Into<Numeric>> Rem<T> for Numeric {
    type Output = Numeric;

    fn rem(mut self, rhs: T) -> Self::Output {
        self %= rhs;
        self
    }
}

impl<T: Into<Numeric>> RemAssign<T> for Numeric {
    fn rem_assign(&mut self, rhs: T) {
        let expr = Expression::expr(format!("Mod({}, {})", self.0.expr, rhs.into().0.expr));
        self.0 = expr;
    }
}

impl From<&Self> for Numeric {
    fn from(value: &Self) -> Self {
        Self(value.0.clone())
    }
}

impl<T: Copy + Into<Numeric>> From<&T> for Numeric {
    fn from(value: &T) -> Self {
        (*value).into()
    }
}

impl<T> PartialEq<T> for Numeric
where
    Numeric: for<'a> From<&'a T>,
{
    fn eq(&self, other: &T) -> bool {
        let other = Self::from(other);
        if self.0.expr == other.0.expr {
            return true;
        }

        if let Ok(v) = self.0.expr.parse::<f64>() {
            if let Ok(u) = other.0.expr.parse::<f64>() {
                return v.partial_cmp(&u).is_some_and(|v| v.is_eq());
            }
        }

        false
    }
}

impl Zero for Numeric {
    fn zero() -> Self {
        Self(Expression::expr("0"))
    }

    /// WARNING: This is not necessarily always precise
    fn is_zero(&self) -> bool {
        self.0.expr.as_str() == "0" || self.0.expr.as_str() == "0.0"
    }
}

impl One for Numeric {
    fn one() -> Self {
        Self(Expression::expr("1"))
    }

    /// WARNING: This is not necessarily always precise
    fn is_one(&self) -> bool {
        self.0.expr.as_str() == "1" || self.0.expr.as_str() == "1.0"
    }
}

impl Bounded for Numeric {
    fn min_value() -> Self {
        Self::zero()
    }

    fn max_value() -> Self {
        Self(Expression::expr(format!("{} + {}i", f64::MAX, f64::MAX)))
    }
}

impl Num for Numeric {
    type FromStrRadixErr = &'static str;

    fn from_str_radix(str: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        if radix != 10 {
            return Err("Only radix of 10 is supported.");
        }

        Ok(Self(Expression::expr(str)))
    }
}

/// Trait for accessing numeric functions
pub trait NumericAccess: Sized
where
    Numeric: From<Self>,
{
    /// Raise the number to a power
    fn pow(self, exponent: impl Into<Numeric>) -> Numeric {
        Numeric::from(self).pow(exponent)
    }

    /// Get the real part of this number
    fn real(self) -> Numeric {
        Numeric::from(self).real()
    }

    /// Get the imaginary part of this number
    fn imaginary(self) -> Numeric {
        Numeric::from(self).imaginary()
    }

    /// Get this complex number's argument
    fn arg(self) -> Numeric {
        Numeric::from(self).arg()
    }
}

impl<T> NumericAccess for T where Numeric: From<Self> {}

/// A circle in the construction
#[derive(Clone)]
pub struct Conic(Expression);

impl Conic {
    /// Set the line's color
    pub fn set_color(&mut self, r: u8, g: u8, b: u8) {
        self.0.style.color = Some(ObjColorType { r, g, b });
    }

    /// Set the line's style
    pub fn set_style(&mut self, style: LineStyle) {
        self.0.style.line_style = Some(style);
    }

    /// Wether to display this line's label
    pub fn set_display_label(&mut self, v: bool) {
        self.0.style.display_label = v;
    }

    /// Default style for a conic
    #[must_use]
    fn style() -> Style {
        Style {
            display_label: false,
            line_style: None,
            color: None,
        }
    }

    /// Create a new circle with a center and a radius
    #[must_use]
    pub fn circle(center: impl Into<Point>, radius: impl Into<Numeric>) -> Self {
        Self(Expression {
            expr: Rc::new(format!(
                "Circle({}, abs({}))",
                center.into().0.expr,
                radius.into().0.expr
            )),
            style: Self::style(),
        })
    }

    /// Get the center of this conic
    #[must_use]
    pub fn center(self) -> Point {
        Point(Expression {
            expr: Rc::new(format!("Center({})", self.0.expr)),
            style: Point::bound(),
        })
    }
}

impl Object for Conic {}

impl From<Var<Conic>> for Conic {
    fn from(value: Var<Conic>) -> Self {
        Self(value.into())
    }
}

impl From<&Var<Conic>> for Conic {
    fn from(value: &Var<Conic>) -> Self {
        Self(value.into())
    }
}

impl From<Conic> for Expression {
    fn from(value: Conic) -> Self {
        value.0
    }
}

impl Expr for Conic {
    type Target = Self;

    fn get_type() -> ElementType {
        ElementType::Conic
    }

    fn var(expr: String) -> Var<Self::Target> {
        Var::new(expr)
    }
}

/// Access for conic functions
pub trait ConicAccess: Sized
where
    Conic: From<Self>,
{
    /// Get the conic's center
    #[must_use]
    fn center(self) -> Point {
        Conic::from(self).center()
    }
}

impl<T> ConicAccess for T where Conic: From<T> {}

/// A ray (half-line)
#[derive(Clone)]
pub struct Ray(Expression);

impl Ray {
    /// Set the line's color
    pub fn set_color(&mut self, r: u8, g: u8, b: u8) {
        self.0.style.color = Some(ObjColorType { r, g, b });
    }

    /// Set the line's style
    pub fn set_style(&mut self, style: LineStyle) {
        self.0.style.line_style = Some(style);
    }

    /// Wether to display this line's label
    pub fn set_display_label(&mut self, v: bool) {
        self.0.style.display_label = v;
    }

    /// Create a ray with an origin, going through a point
    #[must_use]
    pub fn new(origin: impl Into<Point>, through: impl Into<Point>) -> Self {
        Self(Expression::expr(format!(
            "Ray({}, {})",
            origin.into().0.expr,
            through.into().0.expr
        )))
    }
}

impl Object for Ray {}

impl From<Var<Ray>> for Ray {
    fn from(value: Var<Ray>) -> Self {
        Self(value.into())
    }
}

impl From<&Var<Ray>> for Ray {
    fn from(value: &Var<Ray>) -> Self {
        Self(value.into())
    }
}

impl From<Ray> for Expression {
    fn from(value: Ray) -> Self {
        value.0
    }
}

impl Expr for Ray {
    type Target = Self;

    fn get_type() -> ElementType {
        ElementType::Ray
    }

    fn var(expr: String) -> Var<Self::Target> {
        Var::new(expr)
    }
}

impl Addable for Ray {}

/// A segment
#[derive(Clone)]
pub struct Segment(Expression);

impl Segment {
    /// Set the line's color
    pub fn set_color(&mut self, r: u8, g: u8, b: u8) {
        self.0.style.color = Some(ObjColorType { r, g, b });
    }

    /// Set the line's style
    pub fn set_style(&mut self, style: LineStyle) {
        self.0.style.line_style = Some(style);
    }

    /// Wether to display this line's label
    pub fn set_display_label(&mut self, v: bool) {
        self.0.style.display_label = v;
    }

    /// Create a segment connecting two points
    #[must_use]
    pub fn new(a: impl Into<Point>, b: impl Into<Point>) -> Self {
        Self(Expression::expr(format!(
            "Segment({}, {})",
            a.into().0.expr,
            b.into().0.expr
        )))
    }
}

impl Object for Segment {}

impl From<Var<Segment>> for Segment {
    fn from(value: Var<Segment>) -> Self {
        Self(value.into())
    }
}

impl From<&Var<Segment>> for Segment {
    fn from(value: &Var<Segment>) -> Self {
        Self(value.into())
    }
}

impl From<Segment> for Expression {
    fn from(value: Segment) -> Self {
        value.0
    }
}

impl Expr for Segment {
    type Target = Self;

    fn get_type() -> ElementType {
        ElementType::Segment
    }

    fn var(expr: String) -> Var<Self::Target> {
        Var::new(expr)
    }
}

/// Marks this as addable
pub trait Addable {}

impl Addable for Point {}

impl Addable for Line {}

impl Addable for Conic {}

impl Addable for Segment {}

impl Geogebra {
    /// Create an object defined by an expression.
    pub fn add<T: Expr>(&mut self, expr: T, caption: impl ToString) -> Var<T::Target>
    where
        T::Target: Addable,
    {
        let label = self.next_label();
        let expr = expr.into();

        self.data
            .construction
            .items
            .push(ConstructionItem::Expression(raw::Expression {
                type_: T::get_type(),
                label: label.clone(),
                exp: expr.expr.as_ref().clone(),
            }));

        self.data
            .construction
            .items
            .push(ConstructionItem::Element(Element {
                type_: T::get_type(),
                label: label.clone(),
                caption: Some(caption.to_string().into()),
                ..expr.style.to_element()
            }));

        T::var(label)
    }

    /// Add a point with a position hint.
    pub fn add_point(
        &mut self,
        point: impl Into<Point>,
        caption: impl ToString,
        (x, y): (f64, f64),
    ) -> Var<Point> {
        let label = self.next_label();
        let point = point.into();

        self.data
            .construction
            .items
            .push(ConstructionItem::Expression(raw::Expression {
                type_: ElementType::Point,
                label: label.clone(),
                exp: point.0.expr.as_ref().clone(),
            }));

        self.data
            .construction
            .items
            .push(ConstructionItem::Element(Element {
                type_: ElementType::Point,
                label: label.clone(),
                caption: Some(caption.to_string().into()),
                coords: Some(Coords::xy(x, y)),
                ..point.0.style.to_element()
            }));

        Point::var(label)
    }

    /// Make an expression into a variable without making it an element.
    pub fn var<T: Expr>(&mut self, expr: T) -> Var<T::Target> {
        let label = self.next_label();
        let expr = expr.into();

        self.data
            .construction
            .items
            .push(ConstructionItem::Expression(raw::Expression {
                type_: T::get_type(),
                label: label.clone(),
                exp: expr.expr.as_ref().clone(),
            }));

        self.data
            .construction
            .items
            .push(ConstructionItem::Element(Element {
                type_: T::get_type(),
                label: label.clone(),
                caption: None,
                show: Show::none(),
                ..expr.style.to_element()
            }));

        T::var(label)
    }
}

impl Default for Geogebra {
    fn default() -> Self {
        Self::new()
    }
}
