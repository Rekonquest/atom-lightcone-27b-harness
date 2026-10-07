//! Exact formal execution belongs to an engine lane, not to the passive
//! Lightcone. Every primitive below is evaluated by two independently shaped
//! implementations and fails closed on disagreement.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Rational {
    pub numerator: i128,
    pub denominator: i128,
}

impl Rational {
    pub fn new(numerator: i128, denominator: i128) -> Result<Self, FormalError> {
        if denominator == 0 {
            return Err(FormalError::DivisionByZero);
        }
        if numerator == 0 {
            return Ok(Self {
                numerator: 0,
                denominator: 1,
            });
        }
        let common = i128::try_from(gcd(numerator.unsigned_abs(), denominator.unsigned_abs()))
            .map_err(|_| FormalError::Overflow)?;
        let mut numerator = numerator / common;
        let mut denominator = denominator / common;
        if denominator < 0 {
            numerator = numerator.checked_neg().ok_or(FormalError::Overflow)?;
            denominator = denominator.checked_neg().ok_or(FormalError::Overflow)?;
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    pub fn integer(value: i128) -> Self {
        Self {
            numerator: value,
            denominator: 1,
        }
    }

    pub fn checked_add(self, other: Self) -> Result<Self, FormalError> {
        let left = self
            .numerator
            .checked_mul(other.denominator)
            .ok_or(FormalError::Overflow)?;
        let right = other
            .numerator
            .checked_mul(self.denominator)
            .ok_or(FormalError::Overflow)?;
        let denominator = self
            .denominator
            .checked_mul(other.denominator)
            .ok_or(FormalError::Overflow)?;
        Self::new(
            left.checked_add(right).ok_or(FormalError::Overflow)?,
            denominator,
        )
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, FormalError> {
        self.checked_add(Self::new(
            other.numerator.checked_neg().ok_or(FormalError::Overflow)?,
            other.denominator,
        )?)
    }

    pub fn checked_mul(self, other: Self) -> Result<Self, FormalError> {
        Self::new(
            self.numerator
                .checked_mul(other.numerator)
                .ok_or(FormalError::Overflow)?,
            self.denominator
                .checked_mul(other.denominator)
                .ok_or(FormalError::Overflow)?,
        )
    }

    pub fn checked_div(self, other: Self) -> Result<Self, FormalError> {
        if other.numerator == 0 {
            return Err(FormalError::DivisionByZero);
        }
        Self::new(
            self.numerator
                .checked_mul(other.denominator)
                .ok_or(FormalError::Overflow)?,
            self.denominator
                .checked_mul(other.numerator)
                .ok_or(FormalError::Overflow)?,
        )
    }

    pub fn checked_pow(self, exponent: u32) -> Result<Self, FormalError> {
        let mut result = Self::integer(1);
        for _ in 0..exponent {
            result = result.checked_mul(self)?;
        }
        Ok(result)
    }
}

impl std::fmt::Display for Rational {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.denominator == 1 {
            write!(formatter, "{}", self.numerator)
        } else {
            write!(formatter, "{}/{}", self.numerator, self.denominator)
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Point {
    pub x: i128,
    pub y: i128,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "primitive", content = "arguments")]
pub enum Request {
    ArithmeticAdd {
        left: Rational,
        right: Rational,
    },
    ArithmeticSubtract {
        left: Rational,
        right: Rational,
    },
    ArithmeticMultiply {
        left: Rational,
        right: Rational,
    },
    ArithmeticDivide {
        dividend: Rational,
        divisor: Rational,
    },
    ArithmeticGcd {
        left: i128,
        right: i128,
    },
    ArithmeticLcm {
        left: i128,
        right: i128,
    },
    NumberIsPrime {
        value: i128,
    },
    AlgebraSolveLinear {
        coefficient: Rational,
        offset: Rational,
        result: Rational,
    },
    AlgebraSolveTwoByTwo {
        a11: Rational,
        a12: Rational,
        b1: Rational,
        a21: Rational,
        a22: Rational,
        b2: Rational,
    },
    AlgebraPolynomialValue {
        coefficients: Vec<i128>,
        x: i128,
    },
    CalculusPolynomialDerivative {
        coefficients: Vec<i128>,
    },
    CalculusDefiniteIntegral {
        coefficients: Vec<i128>,
        lower: i128,
        upper: i128,
    },
    CombinatoricsBinomial {
        n: u32,
        k: u32,
    },
    DiscreteArithmeticSeriesSum {
        first: i128,
        difference: i128,
        terms: u32,
    },
    GeometryDistanceSquared {
        left: Point,
        right: Point,
    },
    GeometryTriangleTwiceArea {
        a: Point,
        b: Point,
        c: Point,
    },
    GeometryPythagoreanCheck {
        side_a: i128,
        side_b: i128,
        hypotenuse: i128,
    },
    GeometryOrientation {
        a: Point,
        b: Point,
        c: Point,
    },
    GeometryMidpoint {
        left: Point,
        right: Point,
    },
    GeometryPointOnSegment {
        point: Point,
        start: Point,
        end: Point,
    },
    GeometrySegmentsIntersect {
        first_start: Point,
        first_end: Point,
        second_start: Point,
        second_end: Point,
    },
    GeometryPolygonTwiceArea {
        vertices: Vec<Point>,
    },
    GeometryLineIntersection {
        first_start: Point,
        first_end: Point,
        second_start: Point,
        second_end: Point,
    },
    ProbabilityDisjointUnion {
        left: Rational,
        right: Rational,
    },
    ProbabilityConditional {
        intersection: Rational,
        condition: Rational,
    },
    ProbabilityBayesBinary {
        prior: Rational,
        sensitivity: Rational,
        false_positive_rate: Rational,
    },
    WordRateDistance {
        rate: Rational,
        time: Rational,
    },
    WordUnitRate {
        total: Rational,
        units: Rational,
    },
    WordPercentOf {
        part: Rational,
        whole: Rational,
    },
    WordPercentChange {
        original: Rational,
        updated: Rational,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum Value {
    Rational(Rational),
    RationalList(Vec<Rational>),
    Integer(i128),
    IntegerList(Vec<i128>),
    Boolean(bool),
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExecutionReceipt {
    pub runtime: &'static str,
    pub primitive: &'static str,
    pub value: Value,
    pub oracle_value: Value,
    pub oracle_agreement: bool,
    pub exact: bool,
}

pub fn execute_checked(request: &Request) -> Result<ExecutionReceipt, FormalError> {
    let value = runtime(request)?;
    let oracle_value = oracle(request)?;
    if value != oracle_value {
        return Err(FormalError::OracleDisagreement {
            runtime: value,
            oracle: oracle_value,
        });
    }
    Ok(ExecutionReceipt {
        runtime: "lightcone-formal-v1",
        primitive: primitive_name(request),
        value: value.clone(),
        oracle_value,
        oracle_agreement: true,
        exact: true,
    })
}

pub fn primitive_name(request: &Request) -> &'static str {
    match request {
        Request::ArithmeticAdd { .. } => "arithmetic_add",
        Request::ArithmeticSubtract { .. } => "arithmetic_subtract",
        Request::ArithmeticMultiply { .. } => "arithmetic_multiply",
        Request::ArithmeticDivide { .. } => "arithmetic_divide",
        Request::ArithmeticGcd { .. } => "arithmetic_gcd",
        Request::ArithmeticLcm { .. } => "arithmetic_lcm",
        Request::NumberIsPrime { .. } => "number_is_prime",
        Request::AlgebraSolveLinear { .. } => "algebra_solve_linear",
        Request::AlgebraSolveTwoByTwo { .. } => "algebra_solve_two_by_two",
        Request::AlgebraPolynomialValue { .. } => "algebra_polynomial_value",
        Request::CalculusPolynomialDerivative { .. } => "calculus_polynomial_derivative",
        Request::CalculusDefiniteIntegral { .. } => "calculus_definite_integral",
        Request::CombinatoricsBinomial { .. } => "combinatorics_binomial",
        Request::DiscreteArithmeticSeriesSum { .. } => "discrete_arithmetic_series_sum",
        Request::GeometryDistanceSquared { .. } => "geometry_distance_squared",
        Request::GeometryTriangleTwiceArea { .. } => "geometry_triangle_twice_area",
        Request::GeometryPythagoreanCheck { .. } => "geometry_pythagorean_check",
        Request::GeometryOrientation { .. } => "geometry_orientation",
        Request::GeometryMidpoint { .. } => "geometry_midpoint",
        Request::GeometryPointOnSegment { .. } => "geometry_point_on_segment",
        Request::GeometrySegmentsIntersect { .. } => "geometry_segments_intersect",
        Request::GeometryPolygonTwiceArea { .. } => "geometry_polygon_twice_area",
        Request::GeometryLineIntersection { .. } => "geometry_line_intersection",
        Request::ProbabilityDisjointUnion { .. } => "probability_disjoint_union",
        Request::ProbabilityConditional { .. } => "probability_conditional",
        Request::ProbabilityBayesBinary { .. } => "probability_bayes_binary",
        Request::WordRateDistance { .. } => "word_rate_distance",
        Request::WordUnitRate { .. } => "word_unit_rate",
        Request::WordPercentOf { .. } => "word_percent_of",
        Request::WordPercentChange { .. } => "word_percent_change",
    }
}

fn runtime(request: &Request) -> Result<Value, FormalError> {
    match request {
        Request::ArithmeticAdd { left, right } => Ok(Value::Rational(left.checked_add(*right)?)),
        Request::ArithmeticSubtract { left, right } => {
            Ok(Value::Rational(left.checked_sub(*right)?))
        }
        Request::ArithmeticMultiply { left, right } => {
            Ok(Value::Rational(left.checked_mul(*right)?))
        }
        Request::ArithmeticDivide { dividend, divisor } => {
            Ok(Value::Rational(dividend.checked_div(*divisor)?))
        }
        Request::ArithmeticGcd { left, right } => {
            if *left == 0 && *right == 0 {
                return Err(FormalError::Undefined("gcd of zero and zero"));
            }
            Ok(Value::Integer(
                i128::try_from(raw_gcd(left.unsigned_abs(), right.unsigned_abs()))
                    .map_err(|_| FormalError::Overflow)?,
            ))
        }
        Request::ArithmeticLcm { left, right } => {
            if *left == 0 || *right == 0 {
                return Ok(Value::Integer(0));
            }
            let common = i128::try_from(raw_gcd(left.unsigned_abs(), right.unsigned_abs()))
                .map_err(|_| FormalError::Overflow)?;
            let product = left
                .checked_div(common)
                .and_then(|value| value.checked_mul(*right))
                .and_then(i128::checked_abs)
                .ok_or(FormalError::Overflow)?;
            Ok(Value::Integer(product))
        }
        Request::NumberIsPrime { value } => {
            validate_prime_input(*value)?;
            Ok(Value::Boolean(is_prime_runtime(*value)))
        }
        Request::AlgebraSolveLinear {
            coefficient,
            offset,
            result,
        } => {
            if coefficient.numerator == 0 {
                return Err(FormalError::Undefined("linear coefficient is zero"));
            }
            Ok(Value::Rational(
                result.checked_sub(*offset)?.checked_div(*coefficient)?,
            ))
        }
        Request::AlgebraSolveTwoByTwo {
            a11,
            a12,
            b1,
            a21,
            a22,
            b2,
        } => {
            let determinant = a11.checked_mul(*a22)?.checked_sub(a12.checked_mul(*a21)?)?;
            if determinant.numerator == 0 {
                return Err(FormalError::Undefined("two-by-two system is singular"));
            }
            let x = b1
                .checked_mul(*a22)?
                .checked_sub(a12.checked_mul(*b2)?)?
                .checked_div(determinant)?;
            let y = a11
                .checked_mul(*b2)?
                .checked_sub(b1.checked_mul(*a21)?)?
                .checked_div(determinant)?;
            Ok(Value::RationalList(vec![x, y]))
        }
        Request::AlgebraPolynomialValue { coefficients, x } => {
            validate_coefficients(coefficients)?;
            let mut accumulator = 0i128;
            for coefficient in coefficients.iter().rev() {
                accumulator = accumulator
                    .checked_mul(*x)
                    .and_then(|value| value.checked_add(*coefficient))
                    .ok_or(FormalError::Overflow)?;
            }
            Ok(Value::Integer(accumulator))
        }
        Request::CalculusPolynomialDerivative { coefficients } => {
            validate_coefficients(coefficients)?;
            let derivative = coefficients
                .iter()
                .enumerate()
                .skip(1)
                .map(|(power, coefficient)| {
                    coefficient
                        .checked_mul(i128::try_from(power).map_err(|_| FormalError::Overflow)?)
                        .ok_or(FormalError::Overflow)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Value::IntegerList(derivative))
        }
        Request::CalculusDefiniteIntegral {
            coefficients,
            lower,
            upper,
        } => {
            validate_coefficients(coefficients)?;
            let mut total = Rational::integer(0);
            for (power, coefficient) in coefficients.iter().enumerate() {
                let exponent = u32::try_from(power + 1).map_err(|_| FormalError::Overflow)?;
                let upper_term = Rational::integer(*upper).checked_pow(exponent)?;
                let lower_term = Rational::integer(*lower).checked_pow(exponent)?;
                let delta = upper_term.checked_sub(lower_term)?;
                total = total.checked_add(
                    delta.checked_mul(Rational::new(*coefficient, i128::from(exponent))?)?,
                )?;
            }
            Ok(Value::Rational(total))
        }
        Request::CombinatoricsBinomial { n, k } => {
            validate_binomial(*n, *k)?;
            let reduced = (*k).min(*n - *k);
            let mut result = 1i128;
            for index in 1..=reduced {
                let numerator = i128::from(*n - reduced + index);
                result = result
                    .checked_mul(numerator)
                    .ok_or(FormalError::Overflow)?
                    .checked_div(i128::from(index))
                    .ok_or(FormalError::Overflow)?;
            }
            Ok(Value::Integer(result))
        }
        Request::DiscreteArithmeticSeriesSum {
            first,
            difference,
            terms,
        } => {
            validate_series_terms(*terms)?;
            if *terms == 0 {
                return Ok(Value::Integer(0));
            }
            let count = i128::from(*terms);
            let last = i128::from(*terms - 1)
                .checked_mul(*difference)
                .and_then(|value| first.checked_add(value))
                .ok_or(FormalError::Overflow)?;
            let pair = first.checked_add(last).ok_or(FormalError::Overflow)?;
            let result = if terms.is_multiple_of(2) {
                (count / 2).checked_mul(pair)
            } else {
                count.checked_mul(pair / 2)
            }
            .ok_or(FormalError::Overflow)?;
            Ok(Value::Integer(result))
        }
        Request::GeometryDistanceSquared { left, right } => {
            let dx = left.x.checked_sub(right.x).ok_or(FormalError::Overflow)?;
            let dy = left.y.checked_sub(right.y).ok_or(FormalError::Overflow)?;
            Ok(Value::Integer(
                dx.checked_mul(dx)
                    .and_then(|value| {
                        dy.checked_mul(dy)
                            .and_then(|other| value.checked_add(other))
                    })
                    .ok_or(FormalError::Overflow)?,
            ))
        }
        Request::GeometryTriangleTwiceArea { a, b, c } => {
            let abx = b.x.checked_sub(a.x).ok_or(FormalError::Overflow)?;
            let aby = b.y.checked_sub(a.y).ok_or(FormalError::Overflow)?;
            let acx = c.x.checked_sub(a.x).ok_or(FormalError::Overflow)?;
            let acy = c.y.checked_sub(a.y).ok_or(FormalError::Overflow)?;
            let cross = abx
                .checked_mul(acy)
                .and_then(|left| {
                    aby.checked_mul(acx)
                        .and_then(|right| left.checked_sub(right))
                })
                .ok_or(FormalError::Overflow)?;
            Ok(Value::Integer(
                cross.checked_abs().ok_or(FormalError::Overflow)?,
            ))
        }
        Request::GeometryPythagoreanCheck {
            side_a,
            side_b,
            hypotenuse,
        } => {
            if *side_a <= 0 || *side_b <= 0 || *hypotenuse <= 0 {
                return Err(FormalError::InvalidInput("triangle sides must be positive"));
            }
            let left = side_a
                .checked_mul(*side_a)
                .and_then(|value| {
                    side_b
                        .checked_mul(*side_b)
                        .and_then(|other| value.checked_add(other))
                })
                .ok_or(FormalError::Overflow)?;
            let right = hypotenuse
                .checked_mul(*hypotenuse)
                .ok_or(FormalError::Overflow)?;
            Ok(Value::Boolean(left == right))
        }
        Request::GeometryOrientation { a, b, c } => {
            let abx = b.x.checked_sub(a.x).ok_or(FormalError::Overflow)?;
            let aby = b.y.checked_sub(a.y).ok_or(FormalError::Overflow)?;
            let acx = c.x.checked_sub(a.x).ok_or(FormalError::Overflow)?;
            let acy = c.y.checked_sub(a.y).ok_or(FormalError::Overflow)?;
            let determinant = abx
                .checked_mul(acy)
                .and_then(|left| {
                    aby.checked_mul(acx)
                        .and_then(|right| left.checked_sub(right))
                })
                .ok_or(FormalError::Overflow)?;
            Ok(Value::Integer(determinant.signum()))
        }
        Request::GeometryMidpoint { left, right } => {
            let x = left.x.checked_add(right.x).ok_or(FormalError::Overflow)?;
            let y = left.y.checked_add(right.y).ok_or(FormalError::Overflow)?;
            Ok(Value::RationalList(vec![
                Rational::new(x, 2)?,
                Rational::new(y, 2)?,
            ]))
        }
        Request::GeometryPointOnSegment { point, start, end } => {
            let determinant = orientation_determinant(*start, *end, *point)?;
            Ok(Value::Boolean(
                determinant == 0
                    && coordinate_between(point.x, start.x, end.x)
                    && coordinate_between(point.y, start.y, end.y),
            ))
        }
        Request::GeometrySegmentsIntersect {
            first_start,
            first_end,
            second_start,
            second_end,
        } => {
            let first_left = orientation_determinant(*first_start, *first_end, *second_start)?;
            let first_right = orientation_determinant(*first_start, *first_end, *second_end)?;
            let second_left = orientation_determinant(*second_start, *second_end, *first_start)?;
            let second_right = orientation_determinant(*second_start, *second_end, *first_end)?;
            let proper = opposite_signs(first_left, first_right)
                && opposite_signs(second_left, second_right);
            let boundary = (first_left == 0
                && point_within_box(*second_start, *first_start, *first_end))
                || (first_right == 0 && point_within_box(*second_end, *first_start, *first_end))
                || (second_left == 0 && point_within_box(*first_start, *second_start, *second_end))
                || (second_right == 0 && point_within_box(*first_end, *second_start, *second_end));
            Ok(Value::Boolean(proper || boundary))
        }
        Request::GeometryPolygonTwiceArea { vertices } => {
            validate_polygon(vertices)?;
            let mut signed_twice_area = 0i128;
            for index in 0..vertices.len() {
                let current = vertices[index];
                let next = vertices[(index + 1) % vertices.len()];
                let term = current
                    .x
                    .checked_mul(next.y)
                    .and_then(|left| {
                        current
                            .y
                            .checked_mul(next.x)
                            .and_then(|right| left.checked_sub(right))
                    })
                    .ok_or(FormalError::Overflow)?;
                signed_twice_area = signed_twice_area
                    .checked_add(term)
                    .ok_or(FormalError::Overflow)?;
            }
            Ok(Value::Integer(
                signed_twice_area
                    .checked_abs()
                    .ok_or(FormalError::Overflow)?,
            ))
        }
        Request::GeometryLineIntersection {
            first_start,
            first_end,
            second_start,
            second_end,
        } => {
            validate_line(*first_start, *first_end)?;
            validate_line(*second_start, *second_end)?;
            let x12 = first_start
                .x
                .checked_sub(first_end.x)
                .ok_or(FormalError::Overflow)?;
            let y12 = first_start
                .y
                .checked_sub(first_end.y)
                .ok_or(FormalError::Overflow)?;
            let x34 = second_start
                .x
                .checked_sub(second_end.x)
                .ok_or(FormalError::Overflow)?;
            let y34 = second_start
                .y
                .checked_sub(second_end.y)
                .ok_or(FormalError::Overflow)?;
            let denominator = checked_cross(x12, y12, x34, y34)?;
            if denominator == 0 {
                return Err(FormalError::Undefined(
                    "lines do not have a unique intersection",
                ));
            }
            let first_det = checked_cross(first_start.x, first_start.y, first_end.x, first_end.y)?;
            let second_det =
                checked_cross(second_start.x, second_start.y, second_end.x, second_end.y)?;
            let x_numerator = checked_cross(first_det, x12, second_det, x34)?;
            let y_numerator = checked_cross(first_det, y12, second_det, y34)?;
            Ok(Value::RationalList(vec![
                Rational::new(x_numerator, denominator)?,
                Rational::new(y_numerator, denominator)?,
            ]))
        }
        Request::ProbabilityDisjointUnion { left, right } => {
            validate_probability(*left)?;
            validate_probability(*right)?;
            let total = left.checked_add(*right)?;
            validate_probability(total)?;
            Ok(Value::Rational(total))
        }
        Request::ProbabilityConditional {
            intersection,
            condition,
        } => {
            validate_probability(*intersection)?;
            validate_probability(*condition)?;
            if condition.numerator == 0 {
                return Err(FormalError::Undefined(
                    "conditioning event has probability zero",
                ));
            }
            if !rational_leq(*intersection, *condition)? {
                return Err(FormalError::InvalidInput(
                    "intersection probability exceeds condition probability",
                ));
            }
            Ok(Value::Rational(intersection.checked_div(*condition)?))
        }
        Request::ProbabilityBayesBinary {
            prior,
            sensitivity,
            false_positive_rate,
        } => {
            validate_probability(*prior)?;
            validate_probability(*sensitivity)?;
            validate_probability(*false_positive_rate)?;
            let true_positive = prior.checked_mul(*sensitivity)?;
            let false_positive = Rational::integer(1)
                .checked_sub(*prior)?
                .checked_mul(*false_positive_rate)?;
            let evidence = true_positive.checked_add(false_positive)?;
            if evidence.numerator == 0 {
                return Err(FormalError::Undefined(
                    "positive evidence has probability zero",
                ));
            }
            Ok(Value::Rational(true_positive.checked_div(evidence)?))
        }
        Request::WordRateDistance { rate, time } => Ok(Value::Rational(rate.checked_mul(*time)?)),
        Request::WordUnitRate { total, units } => Ok(Value::Rational(total.checked_div(*units)?)),
        Request::WordPercentOf { part, whole } => Ok(Value::Rational(
            part.checked_div(*whole)?
                .checked_mul(Rational::integer(100))?,
        )),
        Request::WordPercentChange { original, updated } => {
            if original.numerator == 0 {
                return Err(FormalError::Undefined("percent-change base is zero"));
            }
            Ok(Value::Rational(
                updated
                    .checked_sub(*original)?
                    .checked_div(*original)?
                    .checked_mul(Rational::integer(100))?,
            ))
        }
    }
}

fn oracle(request: &Request) -> Result<Value, FormalError> {
    match request {
        Request::ArithmeticAdd { left, right } => Rational::new(
            left.numerator
                .checked_mul(right.denominator)
                .and_then(|value| {
                    right
                        .numerator
                        .checked_mul(left.denominator)
                        .and_then(|other| value.checked_add(other))
                })
                .ok_or(FormalError::Overflow)?,
            left.denominator
                .checked_mul(right.denominator)
                .ok_or(FormalError::Overflow)?,
        )
        .map(Value::Rational),
        Request::ArithmeticSubtract { left, right } => oracle(&Request::ArithmeticAdd {
            left: *left,
            right: Rational::new(
                right.numerator.checked_neg().ok_or(FormalError::Overflow)?,
                right.denominator,
            )?,
        }),
        Request::ArithmeticMultiply { left, right } => Rational::new(
            left.numerator
                .checked_mul(right.numerator)
                .ok_or(FormalError::Overflow)?,
            left.denominator
                .checked_mul(right.denominator)
                .ok_or(FormalError::Overflow)?,
        )
        .map(Value::Rational),
        Request::ArithmeticDivide { dividend, divisor } => {
            if divisor.numerator == 0 {
                return Err(FormalError::DivisionByZero);
            }
            Rational::new(
                dividend
                    .numerator
                    .checked_mul(divisor.denominator)
                    .ok_or(FormalError::Overflow)?,
                dividend
                    .denominator
                    .checked_mul(divisor.numerator)
                    .ok_or(FormalError::Overflow)?,
            )
            .map(Value::Rational)
        }
        Request::ArithmeticGcd { left, right } => {
            if *left == 0 && *right == 0 {
                return Err(FormalError::Undefined("gcd of zero and zero"));
            }
            Ok(Value::Integer(
                i128::try_from(binary_gcd(left.unsigned_abs(), right.unsigned_abs()))
                    .map_err(|_| FormalError::Overflow)?,
            ))
        }
        Request::ArithmeticLcm { left, right } => {
            if *left == 0 || *right == 0 {
                return Ok(Value::Integer(0));
            }
            let common = i128::try_from(binary_gcd(left.unsigned_abs(), right.unsigned_abs()))
                .map_err(|_| FormalError::Overflow)?;
            let result = right
                .checked_div(common)
                .and_then(|value| value.checked_mul(*left))
                .and_then(i128::checked_abs)
                .ok_or(FormalError::Overflow)?;
            let left_abs = left.checked_abs().ok_or(FormalError::Overflow)?;
            let right_abs = right.checked_abs().ok_or(FormalError::Overflow)?;
            if result % left_abs != 0 || result % right_abs != 0 {
                return Err(FormalError::Undefined("lcm failed divisibility check"));
            }
            Ok(Value::Integer(result))
        }
        Request::NumberIsPrime { value } => {
            validate_prime_input(*value)?;
            let prime = if *value < 2 {
                false
            } else {
                let candidate = u128::try_from(*value).map_err(|_| FormalError::Overflow)?;
                let mut divisor = 2u128;
                let mut found = false;
                while divisor <= candidate / divisor {
                    if candidate.is_multiple_of(divisor) {
                        found = true;
                        break;
                    }
                    divisor += 1;
                }
                !found
            };
            Ok(Value::Boolean(prime))
        }
        Request::AlgebraSolveLinear {
            coefficient,
            offset,
            result,
        } => {
            if coefficient.numerator == 0 {
                return Err(FormalError::Undefined("linear coefficient is zero"));
            }
            let candidate = result.checked_sub(*offset)?.checked_div(*coefficient)?;
            let check = coefficient.checked_mul(candidate)?.checked_add(*offset)?;
            if check != *result {
                return Err(FormalError::Undefined(
                    "linear solution failed substitution",
                ));
            }
            Ok(Value::Rational(candidate))
        }
        Request::AlgebraSolveTwoByTwo {
            a11,
            a12,
            b1,
            a21,
            a22,
            b2,
        } => {
            let (x, y) = if a11.numerator != 0 {
                let scaled = a21.checked_div(*a11)?;
                let remaining = a22.checked_sub(scaled.checked_mul(*a12)?)?;
                if remaining.numerator == 0 {
                    return Err(FormalError::Undefined("two-by-two system is singular"));
                }
                let y = b2
                    .checked_sub(scaled.checked_mul(*b1)?)?
                    .checked_div(remaining)?;
                let x = b1.checked_sub(a12.checked_mul(y)?)?.checked_div(*a11)?;
                (x, y)
            } else if a12.numerator != 0 && a21.numerator != 0 {
                let y = b1.checked_div(*a12)?;
                let x = b2.checked_sub(a22.checked_mul(y)?)?.checked_div(*a21)?;
                (x, y)
            } else {
                return Err(FormalError::Undefined("two-by-two system is singular"));
            };
            let first_check = a11.checked_mul(x)?.checked_add(a12.checked_mul(y)?)?;
            let second_check = a21.checked_mul(x)?.checked_add(a22.checked_mul(y)?)?;
            if first_check != *b1 || second_check != *b2 {
                return Err(FormalError::Undefined(
                    "two-by-two solution failed substitution",
                ));
            }
            Ok(Value::RationalList(vec![x, y]))
        }
        Request::AlgebraPolynomialValue { coefficients, x } => {
            validate_coefficients(coefficients)?;
            let mut sum = 0i128;
            let mut power = 1i128;
            for coefficient in coefficients {
                sum = sum
                    .checked_add(
                        coefficient
                            .checked_mul(power)
                            .ok_or(FormalError::Overflow)?,
                    )
                    .ok_or(FormalError::Overflow)?;
                power = power.checked_mul(*x).ok_or(FormalError::Overflow)?;
            }
            Ok(Value::Integer(sum))
        }
        Request::CalculusPolynomialDerivative { coefficients } => {
            validate_coefficients(coefficients)?;
            let mut derivative = Vec::new();
            for (power, coefficient) in coefficients.iter().enumerate().skip(1) {
                let factor = i128::try_from(power).map_err(|_| FormalError::Overflow)?;
                derivative.push(
                    coefficient
                        .checked_mul(factor)
                        .ok_or(FormalError::Overflow)?,
                );
            }
            Ok(Value::IntegerList(derivative))
        }
        Request::CalculusDefiniteIntegral {
            coefficients,
            lower,
            upper,
        } => {
            fn antiderivative_at(coefficients: &[i128], x: i128) -> Result<Rational, FormalError> {
                let mut total = Rational::integer(0);
                for (power, coefficient) in coefficients.iter().enumerate() {
                    let exponent = u32::try_from(power + 1).map_err(|_| FormalError::Overflow)?;
                    total = total.checked_add(
                        Rational::new(*coefficient, i128::from(exponent))?
                            .checked_mul(Rational::integer(x).checked_pow(exponent)?)?,
                    )?;
                }
                Ok(total)
            }
            validate_coefficients(coefficients)?;
            Ok(Value::Rational(
                antiderivative_at(coefficients, *upper)?
                    .checked_sub(antiderivative_at(coefficients, *lower)?)?,
            ))
        }
        Request::CombinatoricsBinomial { n, k } => {
            validate_binomial(*n, *k)?;
            let mut row = vec![0i128; usize::try_from(*k + 1).map_err(|_| FormalError::Overflow)?];
            row[0] = 1;
            for level in 1..=*n {
                let top = (*k).min(level);
                for index in (1..=top).rev() {
                    let position = usize::try_from(index).map_err(|_| FormalError::Overflow)?;
                    row[position] = row[position]
                        .checked_add(row[position - 1])
                        .ok_or(FormalError::Overflow)?;
                }
            }
            Ok(Value::Integer(
                row[usize::try_from(*k).map_err(|_| FormalError::Overflow)?],
            ))
        }
        Request::DiscreteArithmeticSeriesSum {
            first,
            difference,
            terms,
        } => {
            validate_series_terms(*terms)?;
            let mut total = 0i128;
            let mut value = *first;
            for _ in 0..*terms {
                total = total.checked_add(value).ok_or(FormalError::Overflow)?;
                value = value
                    .checked_add(*difference)
                    .ok_or(FormalError::Overflow)?;
            }
            Ok(Value::Integer(total))
        }
        Request::GeometryDistanceSquared { left, right } => {
            let expanded = left
                .x
                .checked_mul(left.x)
                .and_then(|value| {
                    right
                        .x
                        .checked_mul(right.x)
                        .and_then(|term| value.checked_add(term))
                })
                .and_then(|value| {
                    left.x
                        .checked_mul(right.x)
                        .and_then(|term| term.checked_mul(2))
                        .and_then(|term| value.checked_sub(term))
                })
                .and_then(|value| {
                    left.y
                        .checked_mul(left.y)
                        .and_then(|term| value.checked_add(term))
                })
                .and_then(|value| {
                    right
                        .y
                        .checked_mul(right.y)
                        .and_then(|term| value.checked_add(term))
                })
                .and_then(|value| {
                    left.y
                        .checked_mul(right.y)
                        .and_then(|term| term.checked_mul(2))
                        .and_then(|term| value.checked_sub(term))
                })
                .ok_or(FormalError::Overflow)?;
            Ok(Value::Integer(expanded))
        }
        Request::GeometryTriangleTwiceArea { a, b, c } => {
            let first =
                a.x.checked_mul(b.y.checked_sub(c.y).ok_or(FormalError::Overflow)?)
                    .ok_or(FormalError::Overflow)?;
            let second =
                b.x.checked_mul(c.y.checked_sub(a.y).ok_or(FormalError::Overflow)?)
                    .ok_or(FormalError::Overflow)?;
            let third =
                c.x.checked_mul(a.y.checked_sub(b.y).ok_or(FormalError::Overflow)?)
                    .ok_or(FormalError::Overflow)?;
            Ok(Value::Integer(
                first
                    .checked_add(second)
                    .and_then(|value| value.checked_add(third))
                    .and_then(i128::checked_abs)
                    .ok_or(FormalError::Overflow)?,
            ))
        }
        Request::GeometryPythagoreanCheck {
            side_a,
            side_b,
            hypotenuse,
        } => {
            if *side_a <= 0 || *side_b <= 0 || *hypotenuse <= 0 {
                return Err(FormalError::InvalidInput("triangle sides must be positive"));
            }
            let mut sides = [*side_a, *side_b, *hypotenuse];
            sides.sort_unstable();
            let check = sides[0]
                .checked_mul(sides[0])
                .and_then(|value| {
                    sides[1]
                        .checked_mul(sides[1])
                        .and_then(|other| value.checked_add(other))
                })
                .ok_or(FormalError::Overflow)?
                == sides[2]
                    .checked_mul(sides[2])
                    .ok_or(FormalError::Overflow)?;
            Ok(Value::Boolean(check))
        }
        Request::GeometryOrientation { a, b, c } => {
            let first =
                a.x.checked_mul(b.y.checked_sub(c.y).ok_or(FormalError::Overflow)?)
                    .ok_or(FormalError::Overflow)?;
            let second =
                b.x.checked_mul(c.y.checked_sub(a.y).ok_or(FormalError::Overflow)?)
                    .ok_or(FormalError::Overflow)?;
            let third =
                c.x.checked_mul(a.y.checked_sub(b.y).ok_or(FormalError::Overflow)?)
                    .ok_or(FormalError::Overflow)?;
            let determinant = first
                .checked_add(second)
                .and_then(|value| value.checked_add(third))
                .ok_or(FormalError::Overflow)?;
            Ok(Value::Integer(determinant.signum()))
        }
        Request::GeometryMidpoint { left, right } => {
            let x = Rational::integer(left.x)
                .checked_add(Rational::integer(right.x))?
                .checked_div(Rational::integer(2))?;
            let y = Rational::integer(left.y)
                .checked_add(Rational::integer(right.y))?
                .checked_div(Rational::integer(2))?;
            Ok(Value::RationalList(vec![x, y]))
        }
        Request::GeometryPointOnSegment { point, start, end } => Ok(Value::Boolean(
            point_on_segment_oracle(*point, *start, *end)?,
        )),
        Request::GeometrySegmentsIntersect {
            first_start,
            first_end,
            second_start,
            second_end,
        } => Ok(Value::Boolean(segments_intersect_parametric(
            *first_start,
            *first_end,
            *second_start,
            *second_end,
        )?)),
        Request::GeometryPolygonTwiceArea { vertices } => {
            validate_polygon(vertices)?;
            let anchor = vertices[0];
            let mut signed_twice_area = 0i128;
            for index in 1..vertices.len() - 1 {
                let left_x = vertices[index]
                    .x
                    .checked_sub(anchor.x)
                    .ok_or(FormalError::Overflow)?;
                let left_y = vertices[index]
                    .y
                    .checked_sub(anchor.y)
                    .ok_or(FormalError::Overflow)?;
                let right_x = vertices[index + 1]
                    .x
                    .checked_sub(anchor.x)
                    .ok_or(FormalError::Overflow)?;
                let right_y = vertices[index + 1]
                    .y
                    .checked_sub(anchor.y)
                    .ok_or(FormalError::Overflow)?;
                signed_twice_area = signed_twice_area
                    .checked_add(checked_cross(left_x, left_y, right_x, right_y)?)
                    .ok_or(FormalError::Overflow)?;
            }
            Ok(Value::Integer(
                signed_twice_area
                    .checked_abs()
                    .ok_or(FormalError::Overflow)?,
            ))
        }
        Request::GeometryLineIntersection {
            first_start,
            first_end,
            second_start,
            second_end,
        } => {
            validate_line(*first_start, *first_end)?;
            validate_line(*second_start, *second_end)?;
            let first_dx = first_end
                .x
                .checked_sub(first_start.x)
                .ok_or(FormalError::Overflow)?;
            let first_dy = first_end
                .y
                .checked_sub(first_start.y)
                .ok_or(FormalError::Overflow)?;
            let second_dx = second_end
                .x
                .checked_sub(second_start.x)
                .ok_or(FormalError::Overflow)?;
            let second_dy = second_end
                .y
                .checked_sub(second_start.y)
                .ok_or(FormalError::Overflow)?;
            let denominator = checked_cross(first_dx, first_dy, second_dx, second_dy)?;
            if denominator == 0 {
                return Err(FormalError::Undefined(
                    "lines do not have a unique intersection",
                ));
            }
            let offset_x = second_start
                .x
                .checked_sub(first_start.x)
                .ok_or(FormalError::Overflow)?;
            let offset_y = second_start
                .y
                .checked_sub(first_start.y)
                .ok_or(FormalError::Overflow)?;
            let parameter = Rational::new(
                checked_cross(offset_x, offset_y, second_dx, second_dy)?,
                denominator,
            )?;
            let x = Rational::integer(first_start.x)
                .checked_add(parameter.checked_mul(Rational::integer(first_dx))?)?;
            let y = Rational::integer(first_start.y)
                .checked_add(parameter.checked_mul(Rational::integer(first_dy))?)?;
            Ok(Value::RationalList(vec![x, y]))
        }
        Request::ProbabilityDisjointUnion { left, right } => {
            validate_probability(*left)?;
            validate_probability(*right)?;
            oracle(&Request::ArithmeticAdd {
                left: *left,
                right: *right,
            })
            .and_then(|value| match value {
                Value::Rational(total) => {
                    validate_probability(total)?;
                    Ok(Value::Rational(total))
                }
                _ => Err(FormalError::Undefined("oracle returned wrong value type")),
            })
        }
        Request::ProbabilityConditional {
            intersection,
            condition,
        } => {
            validate_probability(*intersection)?;
            validate_probability(*condition)?;
            if condition.numerator == 0 {
                return Err(FormalError::Undefined(
                    "conditioning event has probability zero",
                ));
            }
            if !rational_leq(*intersection, *condition)? {
                return Err(FormalError::InvalidInput(
                    "intersection probability exceeds condition probability",
                ));
            }
            let candidate = intersection.checked_div(*condition)?;
            if candidate.checked_mul(*condition)? != *intersection {
                return Err(FormalError::Undefined(
                    "conditional probability failed product check",
                ));
            }
            Ok(Value::Rational(candidate))
        }
        Request::ProbabilityBayesBinary {
            prior,
            sensitivity,
            false_positive_rate,
        } => {
            validate_probability(*prior)?;
            validate_probability(*sensitivity)?;
            validate_probability(*false_positive_rate)?;
            let positive_given_target = prior.checked_mul(*sensitivity)?;
            let positive_given_other = Rational::integer(1)
                .checked_sub(*prior)?
                .checked_mul(*false_positive_rate)?;
            let positive = positive_given_target.checked_add(positive_given_other)?;
            if positive.numerator == 0 {
                return Err(FormalError::Undefined(
                    "positive evidence has probability zero",
                ));
            }
            oracle(&Request::ProbabilityConditional {
                intersection: positive_given_target,
                condition: positive,
            })
        }
        Request::WordRateDistance { rate, time } => oracle(&Request::ArithmeticMultiply {
            left: *rate,
            right: *time,
        }),
        Request::WordUnitRate { total, units } => oracle(&Request::ArithmeticDivide {
            dividend: *total,
            divisor: *units,
        }),
        Request::WordPercentOf { part, whole } => {
            let ratio = match oracle(&Request::ArithmeticDivide {
                dividend: *part,
                divisor: *whole,
            })? {
                Value::Rational(value) => value,
                _ => return Err(FormalError::Undefined("oracle returned wrong value type")),
            };
            oracle(&Request::ArithmeticMultiply {
                left: ratio,
                right: Rational::integer(100),
            })
        }
        Request::WordPercentChange { original, updated } => {
            if original.numerator == 0 {
                return Err(FormalError::Undefined("percent-change base is zero"));
            }
            let percent = updated
                .checked_sub(*original)?
                .checked_div(*original)?
                .checked_mul(Rational::integer(100))?;
            let reconstructed = original.checked_mul(
                Rational::integer(1).checked_add(percent.checked_div(Rational::integer(100))?)?,
            )?;
            if reconstructed != *updated {
                return Err(FormalError::Undefined(
                    "percent change failed reconstruction",
                ));
            }
            Ok(Value::Rational(percent))
        }
    }
}

fn checked_cross(
    left_x: i128,
    left_y: i128,
    right_x: i128,
    right_y: i128,
) -> Result<i128, FormalError> {
    left_x
        .checked_mul(right_y)
        .and_then(|left| {
            left_y
                .checked_mul(right_x)
                .and_then(|right| left.checked_sub(right))
        })
        .ok_or(FormalError::Overflow)
}

fn orientation_determinant(a: Point, b: Point, c: Point) -> Result<i128, FormalError> {
    checked_cross(
        b.x.checked_sub(a.x).ok_or(FormalError::Overflow)?,
        b.y.checked_sub(a.y).ok_or(FormalError::Overflow)?,
        c.x.checked_sub(a.x).ok_or(FormalError::Overflow)?,
        c.y.checked_sub(a.y).ok_or(FormalError::Overflow)?,
    )
}

fn coordinate_between(value: i128, first: i128, second: i128) -> bool {
    first.min(second) <= value && value <= first.max(second)
}

fn point_within_box(point: Point, start: Point, end: Point) -> bool {
    coordinate_between(point.x, start.x, end.x) && coordinate_between(point.y, start.y, end.y)
}

fn opposite_signs(left: i128, right: i128) -> bool {
    (left < 0 && right > 0) || (left > 0 && right < 0)
}

fn validate_polygon(vertices: &[Point]) -> Result<(), FormalError> {
    if !(3..=1024).contains(&vertices.len()) {
        Err(FormalError::InvalidInput(
            "polygon must contain 3..=1024 vertices",
        ))
    } else {
        Ok(())
    }
}

fn validate_line(start: Point, end: Point) -> Result<(), FormalError> {
    if start == end {
        Err(FormalError::InvalidInput(
            "line requires two distinct points",
        ))
    } else {
        Ok(())
    }
}

fn point_on_segment_oracle(point: Point, start: Point, end: Point) -> Result<bool, FormalError> {
    let first = start
        .x
        .checked_mul(end.y.checked_sub(point.y).ok_or(FormalError::Overflow)?)
        .ok_or(FormalError::Overflow)?;
    let second = end
        .x
        .checked_mul(point.y.checked_sub(start.y).ok_or(FormalError::Overflow)?)
        .ok_or(FormalError::Overflow)?;
    let third = point
        .x
        .checked_mul(start.y.checked_sub(end.y).ok_or(FormalError::Overflow)?)
        .ok_or(FormalError::Overflow)?;
    let collinear = first
        .checked_add(second)
        .and_then(|value| value.checked_add(third))
        .ok_or(FormalError::Overflow)?
        == 0;
    let dot = point
        .x
        .checked_sub(start.x)
        .and_then(|left| {
            point
                .x
                .checked_sub(end.x)
                .and_then(|right| left.checked_mul(right))
        })
        .and_then(|x_term| {
            point
                .y
                .checked_sub(start.y)
                .and_then(|left| {
                    point
                        .y
                        .checked_sub(end.y)
                        .and_then(|right| left.checked_mul(right))
                })
                .and_then(|y_term| x_term.checked_add(y_term))
        })
        .ok_or(FormalError::Overflow)?;
    Ok(collinear && dot <= 0)
}

fn unit_interval(value: Rational) -> bool {
    value.numerator >= 0 && value.numerator <= value.denominator
}

fn segments_intersect_parametric(
    first_start: Point,
    first_end: Point,
    second_start: Point,
    second_end: Point,
) -> Result<bool, FormalError> {
    let first_dx = first_end
        .x
        .checked_sub(first_start.x)
        .ok_or(FormalError::Overflow)?;
    let first_dy = first_end
        .y
        .checked_sub(first_start.y)
        .ok_or(FormalError::Overflow)?;
    let second_dx = second_end
        .x
        .checked_sub(second_start.x)
        .ok_or(FormalError::Overflow)?;
    let second_dy = second_end
        .y
        .checked_sub(second_start.y)
        .ok_or(FormalError::Overflow)?;
    if first_dx == 0 && first_dy == 0 {
        return point_on_segment_oracle(first_start, second_start, second_end);
    }
    if second_dx == 0 && second_dy == 0 {
        return point_on_segment_oracle(second_start, first_start, first_end);
    }
    let denominator = checked_cross(first_dx, first_dy, second_dx, second_dy)?;
    let offset_x = second_start
        .x
        .checked_sub(first_start.x)
        .ok_or(FormalError::Overflow)?;
    let offset_y = second_start
        .y
        .checked_sub(first_start.y)
        .ok_or(FormalError::Overflow)?;
    if denominator == 0 {
        if checked_cross(offset_x, offset_y, first_dx, first_dy)? != 0 {
            return Ok(false);
        }
        return Ok(
            coordinate_between(first_start.x, second_start.x, second_end.x)
                && coordinate_between(first_start.y, second_start.y, second_end.y)
                || coordinate_between(first_end.x, second_start.x, second_end.x)
                    && coordinate_between(first_end.y, second_start.y, second_end.y)
                || coordinate_between(second_start.x, first_start.x, first_end.x)
                    && coordinate_between(second_start.y, first_start.y, first_end.y)
                || coordinate_between(second_end.x, first_start.x, first_end.x)
                    && coordinate_between(second_end.y, first_start.y, first_end.y),
        );
    }
    let first_parameter = Rational::new(
        checked_cross(offset_x, offset_y, second_dx, second_dy)?,
        denominator,
    )?;
    let second_parameter = Rational::new(
        checked_cross(offset_x, offset_y, first_dx, first_dy)?,
        denominator,
    )?;
    Ok(unit_interval(first_parameter) && unit_interval(second_parameter))
}

fn validate_coefficients(coefficients: &[i128]) -> Result<(), FormalError> {
    if coefficients.is_empty() || coefficients.len() > 1024 {
        Err(FormalError::InvalidInput(
            "polynomial coefficients must contain 1..=1024 values",
        ))
    } else {
        Ok(())
    }
}

fn validate_probability(value: Rational) -> Result<(), FormalError> {
    if value.numerator < 0 || value.numerator > value.denominator {
        Err(FormalError::InvalidInput(
            "probability must lie within [0, 1]",
        ))
    } else {
        Ok(())
    }
}

fn rational_leq(left: Rational, right: Rational) -> Result<bool, FormalError> {
    let left_scaled = left
        .numerator
        .checked_mul(right.denominator)
        .ok_or(FormalError::Overflow)?;
    let right_scaled = right
        .numerator
        .checked_mul(left.denominator)
        .ok_or(FormalError::Overflow)?;
    Ok(left_scaled <= right_scaled)
}

fn validate_prime_input(value: i128) -> Result<(), FormalError> {
    if value.unsigned_abs() > 10_000_000 {
        Err(FormalError::InvalidInput(
            "primality input magnitude must be at most 10000000",
        ))
    } else {
        Ok(())
    }
}

fn is_prime_runtime(value: i128) -> bool {
    if value < 2 {
        return false;
    }
    let candidate = value as u128;
    if candidate == 2 || candidate == 3 {
        return true;
    }
    if candidate.is_multiple_of(2) || candidate.is_multiple_of(3) {
        return false;
    }
    let mut divisor = 5u128;
    while divisor <= candidate / divisor {
        if candidate.is_multiple_of(divisor) || candidate.is_multiple_of(divisor + 2) {
            return false;
        }
        divisor += 6;
    }
    true
}

fn validate_binomial(n: u32, k: u32) -> Result<(), FormalError> {
    if k > n {
        Err(FormalError::InvalidInput("binomial k must not exceed n"))
    } else if n > 100 {
        Err(FormalError::InvalidInput("binomial n must be at most 100"))
    } else {
        Ok(())
    }
}

fn validate_series_terms(terms: u32) -> Result<(), FormalError> {
    if terms > 1_000_000 {
        Err(FormalError::InvalidInput(
            "arithmetic series terms must be at most 1000000",
        ))
    } else {
        Ok(())
    }
}

fn raw_gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn binary_gcd(mut left: u128, mut right: u128) -> u128 {
    if left == 0 {
        return right;
    }
    if right == 0 {
        return left;
    }
    let common_twos = (left | right).trailing_zeros();
    left >>= left.trailing_zeros();
    loop {
        right >>= right.trailing_zeros();
        if left > right {
            std::mem::swap(&mut left, &mut right);
        }
        right -= left;
        if right == 0 {
            return left << common_twos;
        }
    }
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum FormalError {
    #[error("division by zero")]
    DivisionByZero,
    #[error("exact arithmetic overflow")]
    Overflow,
    #[error("invalid input: {0}")]
    InvalidInput(&'static str),
    #[error("undefined operation: {0}")]
    Undefined(&'static str),
    #[error("formal runtime and oracle disagreed: runtime={runtime:?}, oracle={oracle:?}")]
    OracleDisagreement { runtime: Value, oracle: Value },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(numerator: i128, denominator: i128) -> Rational {
        Rational::new(numerator, denominator).unwrap()
    }

    #[test]
    fn rational_normalization_and_arithmetic_are_exact() {
        assert_eq!(r(-2, -4), r(1, 2));
        assert_eq!(r(1, 3).checked_add(r(1, 6)).unwrap(), r(1, 2));
        assert_eq!(r(7, 9).checked_div(r(14, 3)).unwrap(), r(1, 6));
    }

    #[test]
    fn division_by_zero_fails_closed() {
        assert_eq!(
            r(1, 2).checked_div(Rational::integer(0)),
            Err(FormalError::DivisionByZero)
        );
    }

    #[test]
    fn linear_solution_is_dual_checked() {
        let receipt = execute_checked(&Request::AlgebraSolveLinear {
            coefficient: r(3, 2),
            offset: r(-7, 3),
            result: r(13, 6),
        })
        .unwrap();
        assert_eq!(receipt.value, Value::Rational(r(3, 1)));
        assert!(receipt.oracle_agreement);
    }

    #[test]
    fn polynomial_horner_matches_naive_oracle() {
        let receipt = execute_checked(&Request::AlgebraPolynomialValue {
            coefficients: vec![13, -47, 61, -9],
            x: 17,
        })
        .unwrap();
        assert_eq!(receipt.value, Value::Integer(-27_374));
    }

    #[test]
    fn derivative_is_exact() {
        let receipt = execute_checked(&Request::CalculusPolynomialDerivative {
            coefficients: vec![3, -5, 7, 11],
        })
        .unwrap();
        assert_eq!(receipt.value, Value::IntegerList(vec![-5, 14, 33]));
    }

    #[test]
    fn definite_integral_uses_exact_rationals() {
        let receipt = execute_checked(&Request::CalculusDefiniteIntegral {
            coefficients: vec![3, -5, 7],
            lower: -2,
            upper: 4,
        })
        .unwrap();
        assert_eq!(receipt.value, Value::Rational(r(156, 1)));
    }

    #[test]
    fn geometry_primitives_agree_with_independent_forms() {
        let distance = execute_checked(&Request::GeometryDistanceSquared {
            left: Point { x: -3, y: 7 },
            right: Point { x: 5, y: -2 },
        })
        .unwrap();
        assert_eq!(distance.value, Value::Integer(145));

        let area = execute_checked(&Request::GeometryTriangleTwiceArea {
            a: Point { x: 0, y: 0 },
            b: Point { x: 5, y: 0 },
            c: Point { x: 2, y: 7 },
        })
        .unwrap();
        assert_eq!(area.value, Value::Integer(35));
    }

    #[test]
    fn pythagorean_boundary_rejects_nonpositive_sides() {
        assert_eq!(
            execute_checked(&Request::GeometryPythagoreanCheck {
                side_a: 0,
                side_b: 4,
                hypotenuse: 5,
            }),
            Err(FormalError::InvalidInput("triangle sides must be positive"))
        );
        assert_eq!(
            execute_checked(&Request::GeometryPythagoreanCheck {
                side_a: 3,
                side_b: 4,
                hypotenuse: 5,
            })
            .unwrap()
            .value,
            Value::Boolean(true)
        );
    }

    #[test]
    fn probability_union_checks_bounds() {
        assert_eq!(
            execute_checked(&Request::ProbabilityDisjointUnion {
                left: r(1, 3),
                right: r(1, 6),
            })
            .unwrap()
            .value,
            Value::Rational(r(1, 2))
        );
        assert!(
            execute_checked(&Request::ProbabilityDisjointUnion {
                left: r(3, 4),
                right: r(3, 4),
            })
            .is_err()
        );
    }

    #[test]
    fn typed_word_problems_do_not_need_model_arithmetic() {
        let distance = execute_checked(&Request::WordRateDistance {
            rate: r(55, 1),
            time: r(5, 2),
        })
        .unwrap();
        assert_eq!(distance.value, Value::Rational(r(275, 2)));

        let unit = execute_checked(&Request::WordUnitRate {
            total: r(189, 4),
            units: r(7, 1),
        })
        .unwrap();
        assert_eq!(unit.value, Value::Rational(r(27, 4)));

        let percent = execute_checked(&Request::WordPercentOf {
            part: r(7, 1),
            whole: r(32, 1),
        })
        .unwrap();
        assert_eq!(percent.value, Value::Rational(r(175, 8)));
    }

    #[test]
    fn number_theory_primitives_are_bounded_and_dual_checked() {
        assert_eq!(
            execute_checked(&Request::ArithmeticGcd {
                left: -84,
                right: 30,
            })
            .unwrap()
            .value,
            Value::Integer(6)
        );
        assert_eq!(
            execute_checked(&Request::ArithmeticLcm {
                left: -84,
                right: 30,
            })
            .unwrap()
            .value,
            Value::Integer(420)
        );
        assert_eq!(
            execute_checked(&Request::NumberIsPrime { value: 97 })
                .unwrap()
                .value,
            Value::Boolean(true)
        );
        assert_eq!(
            execute_checked(&Request::NumberIsPrime { value: 221 })
                .unwrap()
                .value,
            Value::Boolean(false)
        );
        assert_eq!(
            execute_checked(&Request::ArithmeticGcd { left: 0, right: 0 }),
            Err(FormalError::Undefined("gcd of zero and zero"))
        );
        assert!(execute_checked(&Request::NumberIsPrime { value: 10_000_001 }).is_err());
    }

    #[test]
    fn two_by_two_system_is_solved_and_substituted() {
        let receipt = execute_checked(&Request::AlgebraSolveTwoByTwo {
            a11: r(2, 1),
            a12: r(3, 1),
            b1: r(13, 1),
            a21: r(5, 1),
            a22: r(-1, 1),
            b2: r(7, 1),
        })
        .unwrap();
        assert_eq!(receipt.value, Value::RationalList(vec![r(2, 1), r(3, 1)]));
        assert!(
            execute_checked(&Request::AlgebraSolveTwoByTwo {
                a11: r(1, 1),
                a12: r(2, 1),
                b1: r(3, 1),
                a21: r(2, 1),
                a22: r(4, 1),
                b2: r(6, 1),
            })
            .is_err()
        );
    }

    #[test]
    fn combinatorics_and_discrete_sum_have_independent_oracles() {
        assert_eq!(
            execute_checked(&Request::CombinatoricsBinomial { n: 52, k: 5 })
                .unwrap()
                .value,
            Value::Integer(2_598_960)
        );
        assert_eq!(
            execute_checked(&Request::DiscreteArithmeticSeriesSum {
                first: 3,
                difference: 2,
                terms: 5,
            })
            .unwrap()
            .value,
            Value::Integer(35)
        );
        assert!(execute_checked(&Request::CombinatoricsBinomial { n: 4, k: 5 }).is_err());
    }

    #[test]
    fn geometric_orientation_handles_both_directions_and_degeneracy() {
        let a = Point { x: 0, y: 0 };
        let b = Point { x: 4, y: 0 };
        assert_eq!(
            execute_checked(&Request::GeometryOrientation {
                a,
                b,
                c: Point { x: 2, y: 3 },
            })
            .unwrap()
            .value,
            Value::Integer(1)
        );
        assert_eq!(
            execute_checked(&Request::GeometryOrientation {
                a,
                b,
                c: Point { x: 2, y: -3 },
            })
            .unwrap()
            .value,
            Value::Integer(-1)
        );
        assert_eq!(
            execute_checked(&Request::GeometryOrientation {
                a,
                b,
                c: Point { x: 2, y: 0 },
            })
            .unwrap()
            .value,
            Value::Integer(0)
        );
    }

    #[test]
    fn midpoint_and_point_on_segment_are_exact() {
        assert_eq!(
            execute_checked(&Request::GeometryMidpoint {
                left: Point { x: -3, y: 4 },
                right: Point { x: 8, y: -1 },
            })
            .unwrap()
            .value,
            Value::RationalList(vec![r(5, 2), r(3, 2)])
        );
        assert_eq!(
            execute_checked(&Request::GeometryPointOnSegment {
                point: Point { x: 3, y: 3 },
                start: Point { x: 0, y: 0 },
                end: Point { x: 6, y: 6 },
            })
            .unwrap()
            .value,
            Value::Boolean(true)
        );
        assert_eq!(
            execute_checked(&Request::GeometryPointOnSegment {
                point: Point { x: 7, y: 7 },
                start: Point { x: 0, y: 0 },
                end: Point { x: 6, y: 6 },
            })
            .unwrap()
            .value,
            Value::Boolean(false)
        );
    }

    #[test]
    fn segment_intersection_handles_crossing_touching_and_disjoint_cases() {
        let cases = [
            (
                Point { x: 0, y: 0 },
                Point { x: 6, y: 6 },
                Point { x: 0, y: 6 },
                Point { x: 6, y: 0 },
                true,
            ),
            (
                Point { x: 0, y: 0 },
                Point { x: 4, y: 0 },
                Point { x: 4, y: 0 },
                Point { x: 9, y: 0 },
                true,
            ),
            (
                Point { x: 0, y: 0 },
                Point { x: 2, y: 0 },
                Point { x: 3, y: 0 },
                Point { x: 5, y: 0 },
                false,
            ),
            (
                Point { x: 1, y: 1 },
                Point { x: 1, y: 1 },
                Point { x: 0, y: 0 },
                Point { x: 2, y: 2 },
                true,
            ),
        ];
        for (first_start, first_end, second_start, second_end, expected) in cases {
            assert_eq!(
                execute_checked(&Request::GeometrySegmentsIntersect {
                    first_start,
                    first_end,
                    second_start,
                    second_end,
                })
                .unwrap()
                .value,
                Value::Boolean(expected)
            );
        }
    }

    #[test]
    fn polygon_area_and_line_intersection_are_exact() {
        assert_eq!(
            execute_checked(&Request::GeometryPolygonTwiceArea {
                vertices: vec![
                    Point { x: 0, y: 0 },
                    Point { x: 5, y: 0 },
                    Point { x: 5, y: 3 },
                    Point { x: 0, y: 3 },
                ],
            })
            .unwrap()
            .value,
            Value::Integer(30)
        );
        assert_eq!(
            execute_checked(&Request::GeometryLineIntersection {
                first_start: Point { x: 0, y: 0 },
                first_end: Point { x: 3, y: 3 },
                second_start: Point { x: 0, y: 2 },
                second_end: Point { x: 4, y: 0 },
            })
            .unwrap()
            .value,
            Value::RationalList(vec![r(4, 3), r(4, 3)])
        );
        assert_eq!(
            execute_checked(&Request::GeometryLineIntersection {
                first_start: Point { x: 0, y: 0 },
                first_end: Point { x: 1, y: 1 },
                second_start: Point { x: 0, y: 1 },
                second_end: Point { x: 1, y: 2 },
            }),
            Err(FormalError::Undefined(
                "lines do not have a unique intersection"
            ))
        );
        assert!(
            execute_checked(&Request::GeometryPolygonTwiceArea {
                vertices: vec![Point { x: 0, y: 0 }, Point { x: 1, y: 0 }],
            })
            .is_err()
        );
    }

    #[test]
    fn conditional_and_bayesian_probabilities_validate_event_bounds() {
        assert_eq!(
            execute_checked(&Request::ProbabilityConditional {
                intersection: r(1, 4),
                condition: r(1, 2),
            })
            .unwrap()
            .value,
            Value::Rational(r(1, 2))
        );
        assert_eq!(
            execute_checked(&Request::ProbabilityBayesBinary {
                prior: r(1, 100),
                sensitivity: r(9, 10),
                false_positive_rate: r(1, 10),
            })
            .unwrap()
            .value,
            Value::Rational(r(1, 12))
        );
        assert!(
            execute_checked(&Request::ProbabilityConditional {
                intersection: r(3, 4),
                condition: r(1, 2),
            })
            .is_err()
        );
    }

    #[test]
    fn percent_change_preserves_its_original_base() {
        assert_eq!(
            execute_checked(&Request::WordPercentChange {
                original: r(80, 1),
                updated: r(100, 1),
            })
            .unwrap()
            .value,
            Value::Rational(r(25, 1))
        );
        assert_eq!(
            execute_checked(&Request::WordPercentChange {
                original: r(100, 1),
                updated: r(80, 1),
            })
            .unwrap()
            .value,
            Value::Rational(r(-20, 1))
        );
    }
}
