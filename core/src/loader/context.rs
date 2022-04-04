// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ast::{DatePattern, Expr, Query};
use crate::output::{ConversionReply, Digits, NotFoundError, NumberParts, QueryError, QueryReply};
use crate::runtime::Substance;
use crate::types::{BaseUnit, BigInt, Dimensionality, Number, Numeric};
use crate::{commands, Value};
use chrono::{DateTime, Local, TimeZone};
use std::collections::{BTreeMap, BTreeSet};

/// The evaluation context that contains unit definitions.
#[derive(Debug)]
pub struct Context {
    pub dimensions: BTreeSet<BaseUnit>,
    pub canonicalizations: BTreeMap<String, String>,
    pub units: BTreeMap<String, Number>,
    pub quantities: BTreeMap<Dimensionality, String>,
    pub reverse: BTreeMap<Dimensionality, String>,
    pub prefixes: Vec<(String, Number)>,
    pub definitions: BTreeMap<String, Expr>,
    pub docs: BTreeMap<String, String>,
    pub categories: BTreeMap<String, String>,
    pub category_names: BTreeMap<String, String>,
    pub datepatterns: Vec<Vec<DatePattern>>,
    pub substances: BTreeMap<String, Substance>,
    pub substance_symbols: BTreeMap<String, String>,
    pub temporaries: BTreeMap<String, Number>,
    pub now: DateTime<Local>,
    pub short_output: bool,
    pub use_humanize: bool,
    pub save_previous_result: bool,
    pub previous_result: Option<Number>,
}

impl Default for Context {
    /// Equivalent to Context::new()
    fn default() -> Self {
        Context::new()
    }
}

impl Context {
    /// Creates a new, empty context
    pub fn new() -> Context {
        Context {
            short_output: false,
            use_humanize: true,
            save_previous_result: false,

            now: Local.timestamp(0, 0),

            dimensions: BTreeSet::new(),
            prefixes: vec![],
            datepatterns: vec![],
            canonicalizations: BTreeMap::new(),
            units: BTreeMap::new(),
            quantities: BTreeMap::new(),
            reverse: BTreeMap::new(),
            definitions: BTreeMap::new(),
            docs: BTreeMap::new(),
            categories: BTreeMap::new(),
            category_names: BTreeMap::new(),
            substances: BTreeMap::new(),
            substance_symbols: BTreeMap::new(),
            temporaries: BTreeMap::new(),
            previous_result: None,
        }
    }

    pub fn set_time(&mut self, time: DateTime<Local>) {
        self.now = time;
    }

    pub fn update_time(&mut self) {
        self.now = Local::now();
    }

    pub fn load_dates(&mut self, mut dates: Vec<Vec<DatePattern>>) {
        self.datepatterns.append(&mut dates)
    }

    /// Given a unit name, returns its value if it exists. Supports SI
    /// prefixes, plurals, bare dimensions like length, and quantities.
    pub fn lookup(&self, name: &str) -> Option<Number> {
        fn inner(ctx: &Context, name: &str) -> Option<Number> {
            if name == "ans" || name == "ANS" || name == "_" {
                return ctx.previous_result.clone();
            }
            if let Some(v) = ctx.temporaries.get(name).cloned() {
                return Some(v);
            }
            if let Some(k) = ctx.dimensions.get(name) {
                return Some(Number::one_unit(k.to_owned()));
            }
            if let Some(v) = ctx.units.get(name).cloned() {
                return Some(v);
            }
            for (unit, quantity) in &ctx.quantities {
                if name == quantity {
                    return Some(Number {
                        value: Numeric::one(),
                        unit: unit.clone(),
                    });
                }
            }
            None
        }

        let outer = |name: &str| -> Option<Number> {
            if let Some(v) = inner(self, name) {
                return Some(v);
            }
            for &(ref pre, ref value) in &self.prefixes {
                if name.starts_with(pre) {
                    if let Some(v) = inner(self, &name[pre.len()..]) {
                        return Some((&v * value).unwrap());
                    }
                }
            }
            None
        };

        let res = outer(name);
        if res.is_some() {
            return res;
        }

        // after so that "ks" is kiloseconds
        if name.ends_with('s') {
            let name = &name[0..name.len() - 1];
            outer(name)
        } else {
            None
        }
    }

    /// Given a unit name, try to return a canonical name (expanding aliases and such)
    pub fn canonicalize(&self, name: &str) -> Option<String> {
        fn inner(ctx: &Context, name: &str) -> Option<String> {
            if let Some(v) = ctx.canonicalizations.get(name) {
                return Some(v.clone());
            }
            if let Some(k) = ctx.dimensions.get(name) {
                return Some((*k.id).clone());
            }
            if let Some(v) = ctx.definitions.get(name) {
                if let Expr::Unit { ref name } = *v {
                    if let Some(r) = ctx.canonicalize(&*name) {
                        return Some(r);
                    } else {
                        return Some(name.clone());
                    }
                } else {
                    // we cannot canonicalize it further
                    return Some(name.to_owned());
                }
            }
            None
        }

        let outer = |name: &str| -> Option<String> {
            if let Some(v) = inner(self, name) {
                return Some(v);
            }
            for &(ref pre, ref val) in &self.prefixes {
                if name.starts_with(pre) {
                    if let Some(v) = inner(self, &name[pre.len()..]) {
                        let mut pre = pre;
                        for &(ref other, ref otherval) in &self.prefixes {
                            if other.len() > pre.len() && val == otherval {
                                pre = other;
                            }
                        }
                        return Some(format!("{}{}", pre, v));
                    }
                }
            }
            None
        };

        let res = outer(name);
        if res.is_some() {
            return res;
        }

        if name.ends_with('s') {
            let name = &name[0..name.len() - 1];
            outer(name)
        } else {
            None
        }
    }

    /// Describes a value's unit, gives true if the unit is reciprocal
    /// (e.g. you should prefix "1.0 / " or replace "multiply" with
    /// "divide" when rendering it).
    pub fn describe_unit(&self, value: &Number) -> (bool, String) {
        use std::io::Write;

        let mut buf = vec![];
        let mut recip = false;
        let square = Number {
            value: Numeric::one(),
            unit: value.unit.clone(),
        }
        .root(2)
        .ok();
        let inverse = (&Number::one()
            / &Number {
                value: Numeric::one(),
                unit: value.unit.clone(),
            })
            .unwrap();
        if let Some(name) = self.quantities.get(&value.unit) {
            write!(buf, "{}", name).unwrap();
        } else if let Some(name) = square.and_then(|square| self.quantities.get(&square.unit)) {
            write!(buf, "{}^2", name).unwrap();
        } else if let Some(name) = self.quantities.get(&inverse.unit) {
            recip = true;
            write!(buf, "{}", name).unwrap();
        } else {
            let helper = |dim: &BaseUnit, pow: i64, buf: &mut Vec<u8>| {
                let unit = Dimensionality::new_dim(dim.clone(), pow);
                if let Some(name) = self.quantities.get(&unit) {
                    write!(buf, " {}", name).unwrap();
                } else {
                    let unit = Dimensionality::base_unit(dim.clone());
                    if let Some(name) = self.quantities.get(&unit) {
                        write!(buf, " {}", name).unwrap();
                    } else {
                        write!(buf, " '{}'", dim).unwrap();
                    }
                    if pow != 1 {
                        write!(buf, "^{}", pow).unwrap();
                    }
                }
            };

            let mut frac = vec![];
            let mut found = false;
            for (dim, &pow) in value.unit.iter() {
                if pow < 0 {
                    frac.push((dim, -pow));
                } else {
                    found = true;
                    helper(dim, pow, &mut buf);
                }
            }
            if !frac.is_empty() {
                if !found {
                    recip = true;
                } else {
                    write!(buf, " /").unwrap();
                }
                for (dim, pow) in frac {
                    let unit = Dimensionality::new_dim(dim.clone(), pow);
                    if let Some(name) = self.quantities.get(&unit) {
                        write!(buf, " {}", name).unwrap();
                    } else {
                        helper(dim, pow, &mut buf);
                    }
                }
            }
            buf.remove(0);
        }

        (recip, String::from_utf8(buf).unwrap())
    }

    pub fn typo_dym<'a>(&'a self, what: &str) -> Option<&'a str> {
        commands::search_internal(self, what, 1).into_iter().next()
    }

    pub fn unknown_unit_err(&self, name: &str) -> NotFoundError {
        NotFoundError {
            got: name.to_owned(),
            suggestion: self.typo_dym(name).map(|x| x.to_owned()),
        }
    }

    pub fn humanize<Tz: chrono::TimeZone>(&self, date: chrono::DateTime<Tz>) -> Option<String> {
        if self.use_humanize {
            crate::parsing::datetime::humanize(date)
        } else {
            None
        }
    }

    /// Takes a parsed definitions.units from
    /// `gnu_units::parse()`. Prints if there are errors in the file.
    pub fn load(&mut self, defs: crate::ast::Defs) {
        crate::loader::load_defs(self, defs)
    }

    /// Evaluates an expression to compute its value, *excluding* `->`
    /// conversions.
    pub fn eval(&self, expr: &Expr) -> Result<Value, QueryError> {
        crate::runtime::eval_expr(self, expr)
    }

    #[deprecated(since = "0.7.0", note = "renamed to eval_query()")]
    pub fn eval_outer(&self, query: &Query) -> Result<QueryReply, QueryError> {
        self.eval_query(query)
    }

    /// Evaluates an expression, include `->` conversions.
    pub fn eval_query(&self, query: &Query) -> Result<QueryReply, QueryError> {
        crate::runtime::eval_query(self, query)
    }

    pub fn show(
        &self,
        raw: &Number,
        bottom: &Number,
        bottom_name: BTreeMap<String, isize>,
        bottom_const: Numeric,
        base: u8,
        digits: Digits,
    ) -> ConversionReply {
        let (exact, approx) = raw.numeric_value(base, digits);
        let bottom_name = bottom_name
            .into_iter()
            .map(|(a, b)| (BaseUnit::new(&*a), b as i64))
            .collect();
        let (num, den) = bottom_const.to_rational();
        ConversionReply {
            value: NumberParts {
                raw_value: Some(raw.clone()),
                exact_value: exact,
                approx_value: approx,
                factor: if num != BigInt::one() {
                    Some(num.to_string())
                } else {
                    None
                },
                divfactor: if den != BigInt::one() {
                    Some(den.to_string())
                } else {
                    None
                },
                unit: Some(Number::unit_to_string(&bottom_name)),
                raw_unit: Some(bottom_name),
                ..bottom.to_parts(self)
            },
        }
    }
}
