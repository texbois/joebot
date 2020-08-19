use crate::{ChainEntry, Datestamp, TextSource};

pub struct Selector {
    pub date_range: Option<(Datestamp, Datestamp)>,
}

impl Selector {
    pub fn new(
        sources: &[TextSource],
        source_query: &str,
        date_range: Option<(Datestamp, Datestamp)>,
    ) -> Result<Self, String> {
        unimplemented!()
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
    pub fn parse(input: &str) -> Result<Self, String> {
        if input.is_empty() {
            return Err("Empty query string".into())
        }

        let mut lexer = QueryLexer::new(input);
        QueryExpression::disjunction(&mut lexer)
    }

    fn disjunction(l: &mut QueryLexer) -> Result<Self, String> {
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

    fn conjunction(l: &mut QueryLexer) -> Result<Self, String> {
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

    fn clause(l: &mut QueryLexer) -> Result<Self, String> {
        if l.char('!') {
            let clause = QueryExpression::clause(l)?;
            Ok(QueryExpression::Negation(Box::new(clause)))
        } else if l.char('(') {
            let clause = QueryExpression::disjunction(l)?;
            if !l.char(')') {
                Err(l.error("Unbalanced parentheses"))
            } else {
                Ok(clause)
            }
        } else {
            let clause = QueryExpression::term(l)?;
            Ok(clause)
        }
    }

    fn term(l: &mut QueryLexer) -> Result<Self, String> {
        let content = l.term();
        if content.is_empty() {
            Err(l.error("Expected an alphanumerical term"))
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

    fn error(&self, description: &str) -> String {
        let before_curr = self.input.len() - self.curr.len();
        format!(
            "{}: \"{}\" ^ here ^ \"{}\"",
            description,
            &self.input[..before_curr],
            self.curr
        )
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
    fn query_parser_err()
    {
        let mut q = QueryExpression::parse("(a | b");
        assert_eq!("Err(\"Unbalanced parentheses: \\\"(a | b\\\" ^ here ^ \\\"\\\"\")", format!("{:?}", q));

        q = QueryExpression::parse("a | #@");
        assert_eq!("Err(\"Expected an alphanumerical term: \\\"a | \\\" ^ here ^ \\\"#@\\\"\")", format!("{:?}", q));

        q = QueryExpression::parse("#@");
        assert_eq!("Err(\"Expected an alphanumerical term: \\\"\\\" ^ here ^ \\\"#@\\\"\")", format!("{:?}", q));

        q = QueryExpression::parse("");
        assert_eq!("Err(\"Empty query string\")", format!("{:?}", q));
    }
}
