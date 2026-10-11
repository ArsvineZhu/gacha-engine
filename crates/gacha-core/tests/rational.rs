use gacha_core::Rational;
use std::str::FromStr;

#[test]
fn parses_decimal_exactly() {
    let p = Rational::from_str("0.008").expect("parse");
    assert_eq!(p, Rational::new(1, 125).expect("ratio"));
}

#[test]
fn arithmetic_is_exact() {
    let a = Rational::from_str("0.008").expect("parse");
    let b = Rational::from_str("0.05").expect("parse");
    let result = a.checked_add(b).expect("add");
    assert_eq!(result, Rational::from_str("0.058").expect("parse"));
}

#[test]
fn large_fractions_remain_exact() {
    let n = u128::MAX;
    let a = Rational::new(n - 1, n).unwrap();
    let b = Rational::new(n - 2, n - 1).unwrap();
    assert_eq!(a.to_f64(), b.to_f64());
    assert!(a > b);
    let small = Rational::new(1, n).unwrap();
    assert_eq!(
        small.checked_add(small).unwrap(),
        Rational::new(2, n).unwrap()
    );
    assert_eq!(a.checked_sub(a).unwrap(), Rational::ZERO);
}
