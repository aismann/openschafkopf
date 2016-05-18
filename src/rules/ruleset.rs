use card::*;
use stich::*;
use rules::*;
use rules::rulesrufspiel::*;
use rules::rulessolo::*;
use rules::rulesramsch::*;

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::io::BufReader;
use std::collections::HashSet;

pub struct SRuleGroup {
    pub m_str_name : String,
    pub m_vecrules : Vec<Box<TRules>>,
}

pub struct SRuleSet {
    pub m_avecrulegroup : [Vec<SRuleGroup>; 4],
    pub m_orulesramsch : Option<Box<TRules>>,
}

pub fn allowed_rules(vecrulegroup: &Vec<SRuleGroup>) -> Vec<&TRules> {
    vecrulegroup.iter().flat_map(|rulegroup| rulegroup.m_vecrules.iter().map(|rules| rules.as_ref())).collect()
}

pub fn create_rulegroup (str_name: &str, vecrules: Vec<Box<TRules>>) -> Option<SRuleGroup> {
    Some(SRuleGroup{
        m_str_name: str_name.to_string(),
        m_vecrules: vecrules
    })
}

pub fn read_ruleset(path: &Path) -> SRuleSet {
    if !path.exists() {
        println!("File {} not found. Creating it.", path.display());
        let mut file = match File::create(&path) {
            Err(why) => panic!("Could not create {}: {}", path.display(), Error::description(&why)),
            Ok(file) => file,
        };
        // TODO: make creation of ruleset file adjustable
        file.write_all(b"rufspiel\n").unwrap();
        file.write_all(b"solo\n").unwrap();
        file.write_all(b"farbwenz\n").unwrap();
        file.write_all(b"wenz\n").unwrap();
    }
    let setstr_rule_name = {
        assert!(path.exists()); 
        let file = match File::open(&path) {
            Err(why) => panic!("Could not open {}: {}", path.display(), Error::description(&why)),
            Ok(file) => file,
        };
        BufReader::new(&file).lines().map(|str| str.unwrap()).collect::<HashSet<_>>()
    };
    SRuleSet {
        m_avecrulegroup : create_playerindexmap(|eplayerindex| {
            setstr_rule_name.iter()
                .filter_map(|str_l| {
                    println!("allowing {} for {}", str_l, eplayerindex);
                    if str_l=="rufspiel" {
                        create_rulegroup(
                            "Rufspiel", 
                            EFarbe::all_values().iter()
                                .filter(|&efarbe| EFarbe::Herz!=*efarbe)
                                .map(|&efarbe| Box::new(SRulesRufspiel{m_eplayerindex: eplayerindex, m_efarbe: efarbe}) as Box<TRules>)
                                .collect()
                        )
                    } else if str_l=="solo" {
                        create_rulegroup("Solo", all_rulessolo(eplayerindex))
                    } else if str_l=="farbwenz" {
                        create_rulegroup("Farbwenz", all_rulesfarbwenz(eplayerindex))
                    } else if str_l=="wenz" {
                        create_rulegroup("Wenz", all_ruleswenz(eplayerindex))
                    } else {
                        None
                    }
                })
                .collect()
        }),
        m_orulesramsch : { 
            if setstr_rule_name.contains("ramsch") {
                Some(Box::new(SRulesRamsch{}))
            } else {
                None
            }
        },
    }
}

