


`astrolang


// Concept declarations
declare "strength" of [p:planet] as {
    set base_strength = 0;
    if (p is "exalted") { set base_strength = base_strength + 2; }
    if (p is "retrograde") { set base_strength = base_strength + 1; }
    if (p is "combust") { set base_strength = base_strength - 1; }
    base_strength
}

declare [p:planet] is "benefic" when 
  (p = JUPITER) or p = VENUS or p = MERCURY or (p = MOON and (p is not "combust"))
then {
}



declare [p:planet] is "in own sign" of [c:chart] when 
    (p = SUN and SIGN of c = LEO) or
    (p = MOON and SIGN of c = CANCER)


declare "chart" of [p:person] as {
    // Define how a chart is created for a person
    // This could include calculating planetary positions, houses, etc.
    set name of c = name of p;
    c
}

// Rule declarations
declare [c:chart] has "Raja Yoga" when 
    (JUPITER of c) in (HOUSE1 of c or HOUSE4 of c or HOUSE7 of c or HOUSE10 of c) and
    VENUS of c in (HOUSE1 of c or HOUSE4 of c or HOUSE7 of c or HOUSE10 of c)
 then {
 set career of c +=5;
}

declare [c:chart] has "Strong Sun in Aries" when 
    (SUN of c) in AERIS and SUN of c = HOUSE1 of c
 then  {
    set career of c +2;
}

declare [c:chart] has "Challenges in Partnership" when 
    SATURN of c in HOUSE1 of c and MARS of c in HOUSE7 of c
then {
    set relationship of c -=2;
}

```

```grammar

WHITESPACE = _{ " " | "\t" | NEWLINE }
NEWLINE    = _{ "\r\n" | "\n" }
COMMENT    = _{ "//" ~ (!NEWLINE ~ ANY)* ~ NEWLINE }

program = { SOI ~ (declaration | COMMENT)* ~ EOI }

declaration = {
    "declare" ~ (concept_declaration | rule_declaration)
}

concept_declaration = {
    STRING ~ "of" ~ "[" ~ parameter ~ "]" ~ "as" ~ block

  | STRING ~ "of" ~ "[" ~ parameter ~ "]" ~ "as" ~ block
}

rule_declaration = {
    "[" ~ parameter ~ "]" ~ "has" ~ STRING ~ "when" ~ condition ~ ("then" ~ block)?
    |  "[" ~ parameter ~ "]" ~ "is" ~ STRING ~ ("of" ~ "[" ~ parameter ~ "]")? ~ "when" ~ condition ~("then" ~ block)? 
}

parameter = { identifier ~ ":" ~ identifier }

block = { "{" ~ statement* ~ (expression)? ~ "}" }

statement = {
    set_statement
  | if_statement
  | expression ~ ";"
}

set_statement = { "set" ~ identifier ~ ("of" ~ identifier)? ~ ("+=" | "=" | "+" |"-="| "-") ~ expression ~ ";" }

if_statement = { "if" ~ "(" ~ condition ~ ")" ~ block }

condition = {
    atom ~ (logical_op ~ atom)*
}

atom = {
    identifier ~ "is" ~ (STRING | "not" ~ STRING)
  | comparison
  | expression ~ "in" ~ "(" ~ expression ~ ("or" ~ expression)* ~ ")"
  | "(" ~ condition ~ ")"
}

logical_op = { "and" | "or" }

comparison = { expression ~ ("=" | "!=" | "<" | ">" | "<=" | ">=" | "in" ) ~ expression }

expression = { term ~ (arithmetic_operator ~ term)* }

term = {
    identifier ~ ("of" ~ identifier)?
  | NUMBER
  | STRING
  | "(" ~ expression ~ ")"
}

arithmetic_operator = { "+" | "-" | "*" | "/" }

identifier = @{ (ASCII_ALPHA_LOWER | ASCII_ALPHA_UPPER) ~ (ASCII_ALPHANUMERIC | "_")* }

NUMBER = @{ ASCII_DIGIT+ ~ ("." ~ ASCII_DIGIT*)? }

STRING = @{ "\"" ~ (!"\"" ~ ANY)* ~ "\"" }

ASCII_ALPHA_LOWER  = _{ 'a'..'z' }
ASCII_ALPHA_UPPER  = _{ 'A'..'Z' }
ASCII_ALPHANUMERIC = _{ ASCII_ALPHA_LOWER | ASCII_ALPHA_UPPER | ASCII_DIGIT }
ASCII_DIGIT        = _{ '0'..'9' }

```