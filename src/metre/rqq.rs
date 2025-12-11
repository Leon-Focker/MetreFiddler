use crate::metre::rqq::RQQ::{Elem, List};

/// A nested list representing an RQQ notation.
#[derive(Debug, Clone)]
pub enum RQQ {
    Elem(f32),
    List(Vec<RQQ>),
}

impl RQQ {
    fn push(&mut self, item: RQQ) {
        match self {
            List(vec) => vec.push(item),
            Elem(num) => *self = List(vec![Elem(*num), item]),
        }
    }

    fn push_recur(&mut self, item: RQQ, lvl: isize) {
        match self {
            List(vec) => match vec.last_mut() {
                Some(elem) => if lvl <= 0 {
                    elem.push(item)
                } else {
                    elem.push_recur(item, lvl - 1)
                },
                None => *self = List(vec![item]),
            }
            Elem(num) => *self = List(vec![Elem(*num), item]),
        }
    }

    // fn print(&self) {
    //     match self {
    //         Elem(num) => print!("{}", num),
    //         List(vec) => {
    //             print!("(");
    //             for (x, item) in vec.iter().enumerate() {
    //                 if x > 0 { print!(" ") }
    //                 item.print();
    //             }
    //             print!(")");
    //         }
    //     }
    // }

    /// Extract the metrical hierarchy from RQQ notation.
    /// 
    /// # Examples
    /// ```
    /// let rqq = parse_rqq(&String::from("(4 (1 1 1 1))")).unwrap();
    /// let gnsm = rqq.to_gnsm().unwrap();
    /// 
    /// assert_eq!(gnsm, vec![1, 0, 0, 0]);
    /// ```
    pub fn to_gnsm(self) -> Result<Vec<usize>, String>{
        match self {
            Elem(_) => Err("rqq.to_gnsm got malformed rqq list".to_string()),
            List(vec) => {
                if vec.len() == 2 {
                    let mut result = vec[1].clone().to_gnsm_aux(1)?;
                    let max = *result.iter().max().unwrap_or(&0);

                    for i in result.iter_mut() {
                        *i = max - *i
                    }

                    Ok(result)
                } else {
                    Err("rqq.to_gnsm got malformed rqq list".to_string())
                }
            }
        }
    }

    fn to_gnsm_aux(self, lvl: usize) -> Result<Vec<usize>, String> {
        let mut ls: Vec<usize> = Vec::new();
        
        if let List(vec) = self {
            for item in vec {
                match item {
                    Elem(_) => ls.push(lvl),
                    List(vec) => {
                        ls.append(&mut vec[1].clone().to_gnsm_aux(lvl + 1)?)
                    },
                }
            }
        } else { 
            return Err("rqq.to_gnsm: second Element is not a list".to_string())    
        }
        
        ls[0] -= 1;
        Ok(ls)
    }
    
    fn no_empty_lists(&self) -> bool {
        match self {  
            Elem(_) => true,
            List(vec) => {
                if vec.is_empty() { false } else {
                    vec.iter().all(|item| item.no_empty_lists())
                }
            },
        }
    }
    
    fn rqq_num_divisions(&self) -> f32 {
        let mut result = 0.0;
        if let List(vec) = self {
            for divs in vec {
                match divs {
                    Elem(val) => result += val,
                    List(vec) => result += 
                        match vec[0] {
                            Elem(val) => val,
                            _ => 0.0
                        }
                }
            }
        }
        result
    }

    pub fn to_durations(&self, parent_dur: f32) -> Result<Vec<f32>, String> {
        match self {
            Elem(val) => Ok(vec![*val / parent_dur]),
            List(vec) => {
                if vec.len() < 2 {
                    return Err("List must have at least two elements".to_string());
                }

                let second_divs = &vec[1];
                let second_divs_vec = match second_divs {
                    List(v) => v,
                    _ => return Err("Expected a List for subdivisions".to_string()),
                };

                let rqqnd = second_divs.rqq_num_divisions();
                let this_dur = match &vec[0] {
                    Elem(val) => *val,
                    _ => return Err("Expected Elem as first item in List".to_string()),
                };

                let pd = (parent_dur * rqqnd) / this_dur;

                let mut result = Vec::new();
                for div in second_divs_vec {
                    result.extend(div.to_durations(pd)?);
                }
                Ok(result)
            }
        }
    }
}

/// Parse a &str to RQQ 
pub fn parse_rqq(input: &str) ->  Result<RQQ, String> {
    if input.is_empty() { return Err("rqq must have at least one element".to_string()); }
    let mut result: RQQ = List(vec![]);
    let mut lvl = -1;
    let mut elements = Vec::new();
    let mut last = 0;
    
    // separate alphanumeric characters from others
    for (index, matched) in input.match_indices(|c: char| !(c.is_alphanumeric() || c == '\'')) {
        if last != index {
            elements.push(&input[last..index]);
        }
        elements.push(matched);
        last = index + matched.len();
    }

    if last < input.len() {
        elements.push(&input[last..]);
    }

    // match elements
    for element in elements {
        match element {
            "(" => {result.push_recur(List(vec![]), lvl); lvl += 1 },
            ")" => lvl -= 1,
            " " => (),
            _ => {
                // keep numbers only
                if let Ok(num) = element.parse::<f32>() {
                    result.push_recur(Elem(num), lvl)
                };
            }
        }
    }
    
    result = match result {
        List(vec) => {
            if vec.is_empty() { return Err("rqq contains no numbers!".to_string()); }
            vec[0].clone()
        },
        Elem(_) => result,
    };
    
    if result.no_empty_lists() {
        Ok(result)
    } else {
        Err("rqq contains malformed rqq list".to_string())
    }
}
