Nova Gramática (Com Precedência de Operadores e sem Recursão à Esquerda)

# 1. Regras para a definição dos programas e das classes

Prog -> Main_C Def_C

Main_C -> 'class' Id '{' 'public' 'static' 'void' 'main' '(' 'String' '[' ']' Id ')' '{' L_com '}' '}'

Def_C -> 'class' Id Def'_C
      | λ

Def'_C -> '{' Def_V Def_M '}' Def_C
       | 'extends' Id '{' Def_V Def_M '}' Def_C
       
# 2. Regras para a definição das variáveis e dos métodos

Def_V -> Type Id ';' Def_V
      | λ

Def_M -> 'public' Type Id '(' Def'_M
      | λ

Def'_M -> Args ')' '{' Def_V L_com 'return' Exp ';' '}' Def_M
       | ')' '{' Def_V L_com 'return' Exp ';' '}' Def_M

# 3. Regras para a definição dos comandos

L_com -> Com L'_com

L'_com -> Com L'_com
       | λ

Com -> Id Com_Ass
    | 'if' '(' Exp ')' '{' L_com '}' I
    | 'while' '(' Exp ')' '{' L_com '}'
    | 'System' '.' 'out' '.' 'println' '(' Exp ')' ';'

Com_Ass -> '=' Exp ';'
        | '[' Exp ']' '=' Exp ';'

I -> 'else' '{' L_com '}'
  | λ

# 4. Regras para a definição dos tipos e argumentos

Type -> 'int' Type'
     | 'boolean'
     | Id

Type' -> '[' ']'
      | λ

Args -> Type Id Args'

Args' -> ',' Type Id Args'
      | λ

# 5. Regras para a definição das expressões

Exp -> And_exp

And_exp -> Rel_exp And'_exp

And'_exp -> '&&' Rel_exp And'_exp
         | λ

Rel_exp -> Add_exp Rel'_exp

Rel'_exp -> '<' Add_exp Rel'_exp
         | λ

Add_exp -> Mul_exp Add'_exp

Add'_exp -> '+' Mul_exp Add'_exp
         | '-' Mul_exp Add'_exp
         | λ

Mul_exp -> Un_exp Mul'_exp

Mul'_exp -> '*' Un_exp Mul'_exp
         | λ

Un_exp -> '!' Un_exp
       | Psf_exp

Psf_exp -> Pri_exp Psf'_exp

Psf'_exp -> '[' Exp ']' Psf'_exp
         | '.' 'length' Psf'_exp
         | '.' Id '(' L_exp ')' Psf'_exp
         | λ

Pri_exp -> '(' Exp ')'
        | 'true'
        | 'false'
        | Id
        | Number
        | 'this'
        | 'new' Id '(' ')'
        | 'new' 'int' '[' Exp ']'

L_exp -> Exp L'_exp
      | λ

L'_exp -> ',' Exp L'_exp
       | λ

# 6. Regras para a definição dos Id's e dos Números

Id -> Letter Word

Letter -> 'a' | ... | 'z' | 'A' | ... | 'Z'

Word -> Letter | Number | '_' | Letter Word | Number Word | '_' Word | λ

Number -> '0' | ... | '9' | '0' Number | ... | '9' Number
