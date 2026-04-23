# Sintaxe de Trabalho 

A GLC abaixo descreve uma variação da linguagem Java, tal linguagem é uma forma enriquecida da WHILE definida por Hoare. 

## 1 Gramática

Prog → MainC DefCl

MainC → 'class' Id '{' 'public' 'static' 'void' 'main' '(' 'String' '[' ']' Id ')' '{' Cmd '}' '}'

DefCl → 'class' Id '{' DefVar DefMet '}' DefCl
      | 'class' Id 'extends' Id '{' DefVar DefMet '}' DefCl
      | λ

DefVar → Type Id ';' DefVar
       | λ

DefMet → 'public' Type Id '(' Args ')' '{' DefVar Cmd 'return' Exp ';' '}' DefMet
       | 'public' Type Id '(' ')' '{' DefVar Cmd 'return' Exp ';' '}' DefMet
       | λ

Type → 'int' '[' ']'
     | 'boolean'
     | 'int'
     | Id

Args → Type Id
     | Type Id ',' Args

Cmd → '{' Cmd '}'
    | 'if' '(' Exp ')' Cmd 'else' Cmd
    | 'while' '(' Exp ')' Cmd
    | 'System' '.' 'out' '.' 'println' '(' Exp ')' ';'
    | Id '=' Exp ';'
    | Id '[' Exp ']' '=' Exp ';'

Exp → Exp '&&' Exp
    | Exp '>' Exp
    | Exp '+' Exp
    | Exp '-' Exp
    | Exp '*' Exp
    | Exp '[' Exp ']'
    | Exp '.' 'length'
    | Exp '.' Exp '(' ListExp ')'
    | 'new' Id '(' ')'
    | '!' Exp
    | '(' Exp ')'
    | 'true'
    | 'false'
    | Id
    | Number
    | 'this'
    | 'new' 'int' '[' Exp ']'

ListExp → Exp ',' ListExp
        | Exp
        | λ

Id → Char Word
   | Char

Char → 'a' | ... | 'z' | 'A' | ... | 'Z'

Word → '_' | Char Word | Number Word | Char | Number

Number → '0' | '1' | ... | '9'
