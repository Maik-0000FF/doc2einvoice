// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The licence a library states: which licences it consists of, and whether
//! the licences whose texts are at hand fulfil it.
//!
//! A stated licence is an SPDX expression. It may ask for several licences at
//! once or offer a choice among them, and only reading it as an expression
//! tells the two apart.

/// The licence a library states.
#[derive(Debug, Clone)]
pub struct Stated {
    /// The statement as an expression; none where it is not one.
    expression: Option<spdx::Expression>,
}

impl Stated {
    /// Reads what a library states as its licence. Older ways of writing an
    /// expression are read as well, as `MIT/Apache-2.0` for a choice.
    pub fn read(stated: &str) -> Self {
        Self {
            expression: spdx::Expression::parse_mode(stated, spdx::ParseMode::LAX).ok(),
        }
    }

    /// Whether the statement is an SPDX expression.
    pub fn is_expression(&self) -> bool {
        self.expression.is_some()
    }

    /// The licences the statement consists of, each once, in the order they
    /// are stated, without the exceptions added to them. None where it is not
    /// an expression.
    ///
    /// | Stated                              | Licences              |
    /// |-------------------------------------|-----------------------|
    /// | `MIT`                               | `MIT`                 |
    /// | `MIT OR Apache-2.0`                 | `MIT`, `Apache-2.0`   |
    /// | `Apache-2.0 AND MIT`                | `Apache-2.0`, `MIT`   |
    /// | `(MIT OR Apache-2.0) AND MIT`       | `MIT`, `Apache-2.0`   |
    /// | `MIT/Apache-2.0`                    | `MIT`, `Apache-2.0`   |
    /// | `Apache-2.0 WITH LLVM-exception`    | `Apache-2.0`          |
    /// | `GPL-3.0-or-later`                  | `GPL-3.0-or-later`    |
    /// | `SEE LICENSE IN LICENSE`            | none                  |
    /// | `MIT OR`, empty                     | none                  |
    pub fn licenses(&self) -> Vec<String> {
        let mut licenses = Vec::new();
        for requirement in self
            .expression
            .iter()
            .flat_map(spdx::Expression::requirements)
        {
            let license = requirement.req.license.to_string();
            if !licenses.contains(&license) {
                licenses.push(license);
            }
        }
        licenses
    }

    /// The one licence the statement consists of, if it is one licence.
    pub fn only(&self) -> Option<String> {
        match self.licenses().as_slice() {
            [license] => Some(license.clone()),
            _ => None,
        }
    }

    /// Whether the allowed licences can fulfil the statement at all. A
    /// statement that is no expression cannot be asked; it is not refused
    /// here.
    pub fn can_be_allowed(&self, allowed: &[String]) -> bool {
        self.expression.as_ref().is_none_or(|expression| {
            expression.evaluate(|requirement| allowed.contains(&requirement.to_string()))
        })
    }

    /// Whether the licences whose texts are at hand fulfil the statement.
    /// Only a licence that is allowed counts.
    ///
    /// | Stated                             | At hand             | Fulfilled |
    /// |------------------------------------|---------------------|-----------|
    /// | `MIT`                              | `MIT`               | yes       |
    /// | `MIT`                              | none                | no        |
    /// | `MIT`                              | `Zlib`              | no        |
    /// | `MIT OR Apache-2.0`                | `Apache-2.0`        | yes       |
    /// | `Apache-2.0 AND MIT`               | `MIT`               | no        |
    /// | `Apache-2.0 AND MIT`               | both                | yes       |
    /// | `(MIT OR Apache-2.0) AND Zlib`     | `Apache-2.0`, `Zlib` | yes      |
    /// | `(MIT OR Apache-2.0) AND Zlib`     | `MIT`, `Apache-2.0` | no        |
    /// | `Apache-2.0 WITH LLVM-exception`   | `Apache-2.0`        | yes       |
    /// | `MIT`, and MIT is not allowed      | `MIT`               | no        |
    /// | not an expression                  | any licence         | yes       |
    /// | not an expression                  | none                | no        |
    pub fn fulfilled_by(&self, at_hand: &[String], allowed: &[String]) -> bool {
        match &self.expression {
            Some(expression) => expression.evaluate(|requirement| {
                at_hand.contains(&requirement.license.to_string())
                    && allowed.contains(&requirement.to_string())
            }),
            // What cannot be read asks for nothing that could be checked: a
            // text someone named for it is taken at their word.
            None => !at_hand.is_empty(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Stated;

    fn owned(licenses: &[&str]) -> Vec<String> {
        licenses.iter().map(|&license| license.to_owned()).collect()
    }

    #[test]
    fn the_licences_of_a_statement_follow_their_table() {
        let table: [(&str, &[&str], bool); 10] = [
            ("MIT", &["MIT"], true),
            ("MIT OR Apache-2.0", &["MIT", "Apache-2.0"], true),
            ("Apache-2.0 AND MIT", &["Apache-2.0", "MIT"], true),
            ("(MIT OR Apache-2.0) AND MIT", &["MIT", "Apache-2.0"], true),
            ("MIT/Apache-2.0", &["MIT", "Apache-2.0"], true),
            ("Apache-2.0 WITH LLVM-exception", &["Apache-2.0"], true),
            ("GPL-3.0-or-later", &["GPL-3.0-or-later"], true),
            ("SEE LICENSE IN LICENSE", &[], false),
            ("MIT OR", &[], false),
            ("", &[], false),
        ];
        for (stated, expected, is_expression) in table {
            let read = Stated::read(stated);

            assert_eq!(read.licenses(), owned(expected), "{stated}");
            assert_eq!(read.is_expression(), is_expression, "{stated}");
        }
    }

    #[test]
    fn only_one_licence_is_the_only_one() {
        assert_eq!(Stated::read("MIT").only(), Some("MIT".to_owned()));
        assert_eq!(Stated::read("MIT OR Zlib").only(), None);
        assert_eq!(Stated::read("SEE LICENSE IN LICENSE").only(), None);
    }

    #[test]
    fn whether_a_statement_is_fulfilled_follows_its_table() {
        let allowed = owned(&[
            "MIT",
            "Apache-2.0",
            "Apache-2.0 WITH LLVM-exception",
            "Zlib",
        ]);
        let without_mit = owned(&["Apache-2.0", "Zlib"]);
        let choice_and_one = "(MIT OR Apache-2.0) AND Zlib";
        let table: [(&str, &[&str], &[String], bool); 12] = [
            ("MIT", &["MIT"], &allowed, true),
            ("MIT", &[], &allowed, false),
            ("MIT", &["Zlib"], &allowed, false),
            ("MIT OR Apache-2.0", &["Apache-2.0"], &allowed, true),
            ("Apache-2.0 AND MIT", &["MIT"], &allowed, false),
            ("Apache-2.0 AND MIT", &["MIT", "Apache-2.0"], &allowed, true),
            (choice_and_one, &["Apache-2.0", "Zlib"], &allowed, true),
            (choice_and_one, &["MIT", "Apache-2.0"], &allowed, false),
            (
                "Apache-2.0 WITH LLVM-exception",
                &["Apache-2.0"],
                &allowed,
                true,
            ),
            ("MIT", &["MIT"], &without_mit, false),
            ("SEE LICENSE IN LICENSE", &["MIT"], &allowed, true),
            ("SEE LICENSE IN LICENSE", &[], &allowed, false),
        ];
        for (stated, at_hand, allowed, expected) in table {
            assert_eq!(
                Stated::read(stated).fulfilled_by(&owned(at_hand), allowed),
                expected,
                "{stated} with {at_hand:?}"
            );
        }
    }

    #[test]
    fn whether_a_statement_can_be_allowed_follows_what_is_allowed() {
        let allowed = owned(&["MIT", "Zlib"]);
        let table = [
            ("MIT", true),
            ("MIT OR Apache-2.0", true),
            ("MIT AND Apache-2.0", false),
            ("Apache-2.0", false),
            ("Apache-2.0 WITH LLVM-exception", false),
            ("SEE LICENSE IN LICENSE", true),
        ];
        for (stated, expected) in table {
            assert_eq!(
                Stated::read(stated).can_be_allowed(&allowed),
                expected,
                "{stated}"
            );
        }
    }
}
