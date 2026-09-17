use lelwel_python2::generate_syntax_tree;
use pretty_assertions::assert_eq;

macro_rules! check {
    ($file:literal) => {
        let res =
            generate_syntax_tree(&include_str!(concat!("data/", $file, ".py")).replace('\r', ""));
        assert_eq!(
            include_str!(concat!("data/", $file, ".tree")).replace('\r', ""),
            format!("{}", res[0])
        );
        assert_eq!(
            include_str!(concat!("data/", $file, ".diag")).replace('\r', ""),
            format!("{}", res[1])
        );
    };
}

#[test]
fn incomplete() {
    check!("incomplete");
}
