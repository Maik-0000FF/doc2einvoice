// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! What a document is validated as.

use crate::rule_set::RuleSet;
use crate::syntax::Syntax;

/// The specification a document is validated against.
///
/// A profile decides which rule sets apply. Supporting a further specification
/// means adding a variant here and its rule sets in [`Profile::rule_sets`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Profile {
    /// The European standard EN 16931 on its own.
    En16931,
    /// The German XRechnung: EN 16931 and the additional XRechnung rules.
    XRechnung,
}

impl Profile {
    /// Every profile, the one place they are listed.
    pub const ALL: [Self; 2] = [Self::En16931, Self::XRechnung];

    /// The rule sets a document of the given syntax must satisfy, in the order
    /// they are applied.
    #[must_use]
    pub fn rule_sets(self, syntax: Syntax) -> &'static [RuleSet] {
        match (self, syntax) {
            (Self::En16931, Syntax::Ubl) => &[RuleSet::En16931Ubl],
            (Self::En16931, Syntax::Cii) => &[RuleSet::En16931Cii],
            (Self::XRechnung, Syntax::Ubl) => &[RuleSet::En16931Ubl, RuleSet::XRechnungUbl],
            (Self::XRechnung, Syntax::Cii) => &[RuleSet::En16931Cii, RuleSet::XRechnungCii],
        }
    }
}

impl Profile {
    /// Whether validating as this profile applies every rule set that
    /// validating as the other one would, in every syntax.
    #[must_use]
    pub fn applies_the_rules_of(self, other: Self) -> bool {
        Syntax::ALL.into_iter().all(|syntax| {
            other
                .rule_sets(syntax)
                .iter()
                .all(|rule_set| self.rule_sets(syntax).contains(rule_set))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Profile;
    use crate::syntax::Syntax;

    #[test]
    fn xrechnung_applies_the_rules_of_the_standard_and_not_the_other_way_round() {
        assert!(Profile::XRechnung.applies_the_rules_of(Profile::En16931));
        assert!(!Profile::En16931.applies_the_rules_of(Profile::XRechnung));
        for profile in Profile::ALL {
            assert!(profile.applies_the_rules_of(profile));
        }
    }

    #[test]
    fn every_rule_set_of_a_profile_validates_the_syntax_of_the_document() {
        for profile in Profile::ALL {
            for syntax in Syntax::ALL {
                for rule_set in profile.rule_sets(syntax) {
                    assert_eq!(rule_set.info().syntax, syntax, "{profile:?} {rule_set:?}");
                }
            }
        }
    }

    #[test]
    fn xrechnung_includes_the_rules_of_the_standard() {
        for syntax in Syntax::ALL {
            let standard = Profile::En16931.rule_sets(syntax);

            assert!(Profile::XRechnung.rule_sets(syntax).starts_with(standard));
        }
    }
}
