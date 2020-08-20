use crate::{ChainEntry, Datestamp, MarkovChain, TextSource};
use std::collections::{HashMap, HashSet};

pub struct Selector<'a> {
    date_range: Option<(Datestamp, Datestamp)>,
    query: QueryExpression,
    source_to_term_map: HashMap<&'a TextSource, String>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum SelectorError {
    EmptyQuery,
    UnknownTerm { term: String },
    ParserUnbalancedParentheses { location: String },
    ParserExpectedTerm { location: String },
}

impl<'a> Selector<'a> {
    pub fn new(
        chain: &'a MarkovChain,
        source_query_str: &str,
        date_range: Option<(Datestamp, Datestamp)>,
    ) -> Result<Self, SelectorError> {
        let query = QueryExpression::parse(source_query_str)?;
        let terms = query.unique_terms();

        let mut source_to_term_map = HashMap::new();
        for term in terms {
            let source = chain.sources.iter().find(|s| s.names.contains(term));
            if let Some(s) = source {
                source_to_term_map.insert(s, term.to_owned());
            } else {
                return Err(SelectorError::UnknownTerm {
                    term: term.to_owned(),
                });
            }
        }

        Ok(Self {
            date_range,
            query,
            source_to_term_map,
        })
    }

    pub fn sources(&self) -> Vec<&TextSource> {
        self.source_to_term_map.keys().copied().collect()
    }

    pub fn matches_query(&self, used_sources: HashSet<&TextSource>) -> bool {
        let used_terms = used_sources
            .into_iter()
            .map(|s| self.source_to_term_map[s].as_str())
            .collect::<HashSet<_>>();
        self.query.eval(&used_terms)
    }

    pub fn filter_entry(&self, e: &ChainEntry) -> bool {
        if let Some((min_date, max_date)) = self.date_range {
            e.datestamp >= min_date && e.datestamp <= max_date
        } else {
            true
        }
    }
}

// query = disjunction ;
// disjunction = conjunction , { "|" , conjunction } ;
// conjunction = clause , { "&" , clause } ;
// clause = "!" , clause
//        | "(" , disjunction , ")"
//        | term ;
// term = [A-Za-z0-9]([A-Za-z0-9 ]+[A-Za-z0-9])? ;

#[derive(Eq, PartialEq)]
pub enum QueryExpression {
    Disjunction(Box<QueryExpression>, Box<QueryExpression>),
    Conjunction(Box<QueryExpression>, Box<QueryExpression>),
    Negation(Box<QueryExpression>),
    Term(String),
}

impl std::fmt::Debug for QueryExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryExpression::Disjunction(a, b) => write!(f, "({:?}) | ({:?})", a, b),
            QueryExpression::Conjunction(a, b) => write!(f, "({:?}) & ({:?})", a, b),
            QueryExpression::Negation(c) => write!(f, "!({:?})", c),
            QueryExpression::Term(t) => f.write_str(t),
        }
    }
}

impl QueryExpression {
    pub fn parse(input: &str) -> Result<Self, SelectorError> {
        if input.is_empty() {
            return Err(SelectorError::EmptyQuery);
        }

        let mut lexer = QueryLexer::new(input);
        QueryExpression::disjunction(&mut lexer)
    }

    pub fn eval(&self, used_terms: &HashSet<&str>) -> bool {
        match self {
            QueryExpression::Disjunction(a, b) => a.eval(used_terms) || b.eval(used_terms),
            QueryExpression::Conjunction(a, b) => a.eval(used_terms) && b.eval(used_terms),
            QueryExpression::Negation(c) => !c.eval(used_terms),
            QueryExpression::Term(t) => used_terms.contains(&t.as_str()),
        }
    }

    pub fn unique_terms(&self) -> HashSet<&str> {
        fn iter<'a>(q: &'a QueryExpression, term_set: &mut HashSet<&'a str>) {
            match q {
                QueryExpression::Disjunction(a, b) | QueryExpression::Conjunction(a, b) => {
                    iter(&*a, term_set);
                    iter(&*b, term_set);
                }
                QueryExpression::Negation(c) => iter(&*c, term_set),
                QueryExpression::Term(t) => {
                    term_set.insert(&t);
                }
            }
        };

        let mut term_set = HashSet::new();
        iter(self, &mut term_set);
        term_set
    }

    fn disjunction(l: &mut QueryLexer) -> Result<Self, SelectorError> {
        let lhs = QueryExpression::conjunction(l)?;
        let mut rhs: Option<QueryExpression> = None;
        while l.char('|') {
            let next_rhs = QueryExpression::conjunction(l)?;
            rhs = Some(if let Some(curr_rhs) = rhs {
                QueryExpression::Disjunction(Box::new(curr_rhs), Box::new(next_rhs))
            } else {
                next_rhs
            });
        }
        if let Some(r) = rhs {
            Ok(QueryExpression::Disjunction(Box::new(lhs), Box::new(r)))
        } else {
            Ok(lhs)
        }
    }

    fn conjunction(l: &mut QueryLexer) -> Result<Self, SelectorError> {
        let lhs = QueryExpression::clause(l)?;
        let mut rhs: Option<QueryExpression> = None;
        while l.char('&') {
            let next_rhs = QueryExpression::clause(l)?;
            rhs = Some(if let Some(curr_rhs) = rhs {
                QueryExpression::Conjunction(Box::new(curr_rhs), Box::new(next_rhs))
            } else {
                next_rhs
            });
        }
        if let Some(r) = rhs {
            Ok(QueryExpression::Conjunction(Box::new(lhs), Box::new(r)))
        } else {
            Ok(lhs)
        }
    }

    fn clause(l: &mut QueryLexer) -> Result<Self, SelectorError> {
        if l.char('!') {
            let clause = QueryExpression::clause(l)?;
            Ok(QueryExpression::Negation(Box::new(clause)))
        } else if l.char('(') {
            let clause = QueryExpression::disjunction(l)?;
            if !l.char(')') {
                Err(SelectorError::ParserUnbalancedParentheses {
                    location: l.error_location(),
                })
            } else {
                Ok(clause)
            }
        } else {
            let clause = QueryExpression::term(l)?;
            Ok(clause)
        }
    }

    fn term(l: &mut QueryLexer) -> Result<Self, SelectorError> {
        let content = l.term();
        if content.is_empty() {
            Err(SelectorError::ParserExpectedTerm {
                location: l.error_location(),
            })
        } else {
            Ok(QueryExpression::Term(content.to_owned()))
        }
    }
}

struct QueryLexer<'a> {
    input: &'a str,
    curr: &'a str,
}

impl<'a> QueryLexer<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, curr: input }
    }

    fn error_location(&self) -> String {
        let at = self.input.len() - self.curr.len();
        format!("\"{}\" ^ here ^ \"{}\"", &self.input[..at], self.curr)
    }

    fn char(&mut self, c: char) -> bool {
        self.curr = self.curr.trim_start();
        let matches = self.curr.chars().nth(0) == Some(c);
        if matches {
            self.curr = &self.curr[c.len_utf8()..];
        }
        matches
    }

    fn term(&mut self) -> &'a str {
        self.curr = self.curr.trim_start();
        match self
            .curr
            .char_indices()
            .find(|(_, c)| !c.is_alphanumeric() && *c != ' ')
        {
            Some((end_pos, _)) => {
                let term = &self.curr[..end_pos];
                self.curr = &self.curr[end_pos..];
                term.trim_end()
            }
            _ => {
                let term = self.curr;
                self.curr = "";
                term.trim_end()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_parser() {
        let mut q = QueryExpression::parse("юник0д").unwrap();
        assert_eq!("юник0д", format!("{:?}", q));

        q = QueryExpression::parse("юник0д b & юник0д d | 3 & 5").unwrap();
        assert_eq!(
            "((юник0д b) & (юник0д d)) | ((3) & (5))",
            format!("{:?}", q)
        );

        q = QueryExpression::parse("!(3 & 5)").unwrap();
        assert_eq!("!((3) & (5))", format!("{:?}", q));

        q = QueryExpression::parse("a | b | c | d").unwrap();
        assert_eq!("(a) | (((b) | (c)) | (d))", format!("{:?}", q));

        q = QueryExpression::parse("a & b & c & d").unwrap();
        assert_eq!("(a) & (((b) & (c)) & (d))", format!("{:?}", q));

        q = QueryExpression::parse("a | b & c | d").unwrap();
        assert_eq!("(a) | (((b) & (c)) | (d))", format!("{:?}", q));

        q = QueryExpression::parse("(a | b) & (c | d)").unwrap();
        assert_eq!("((a) | (b)) & ((c) | (d))", format!("{:?}", q));
    }

    #[test]
    fn query_parser_err() {
        use SelectorError::*;

        let mut q = QueryExpression::parse("(a | b");
        assert_eq!(
            Err(ParserUnbalancedParentheses {
                location: "\"(a | b\" ^ here ^ \"\"".into()
            }),
            q
        );

        q = QueryExpression::parse("a | #@");
        assert_eq!(
            Err(ParserExpectedTerm {
                location: "\"a | \" ^ here ^ \"#@\"".into()
            }),
            q
        );

        q = QueryExpression::parse("#@");
        assert_eq!(
            Err(ParserExpectedTerm {
                location: "\"\" ^ here ^ \"#@\"".into()
            }),
            q
        );

        q = QueryExpression::parse("");
        assert_eq!(Err(EmptyQuery), q);
    }

    #[test]
    fn test_query_eval() {
        let q = QueryExpression::parse("(a | b) & (c | d)").unwrap();
        let used_terms = &["a", "b"].iter().map(|s| s.to_owned()).collect::<HashSet<_>>();
        assert_eq!(false, q.eval(&used_terms));

        let used_terms = &["a", "d"].iter().map(|s| s.to_owned()).collect::<HashSet<_>>();
        assert_eq!(true, q.eval(&used_terms));
    }
}
