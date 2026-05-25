# Gramática transformada

Gramática de `specs/gramatica.md` após remoção de recursão à esquerda e
fatoração à esquerda, conforme o item 1 da spec do trabalho.

## Mudanças

1. **`Exp` left-recursivo** estratificado em níveis de precedência
   (`ExpAnd → ExpRel → ExpAdd → ExpMul → ExpUnary → ExpPostfix → ExpPrimary`),
   eliminando a recursão à esquerda e preservando a precedência
   matemática (`*` > `+` > `<,>` > `&&`).
2. **Sequências de comandos** — a gramática original tem `MainC` e `DefMet` com
   apenas um único `Cmd` no corpo; isso impede `bubblesort.ling` de parsear (vários
   statements consecutivos). Introduzido o não-terminal `Cmds` para sequências.
3. **`Type`** fatorado à esquerda no `int` (vs `int [ ]`).
4. **`DefMet`** fatorado: `Args` agora aceita `λ`.
5. **`Cmd` começando com `Id`** fatorado entre `Id =` e `Id [ ... ] =`.
6. **`ExpBase` com `'new'`** fatorado entre `new Id ( )` e `new int [ Exp ]`.
7. **`ListExp`** fatorado e a alternativa `λ` movida para o nível do chamador.
8. **`<` adicionado como operador relacional** — não consta na `gramatica.md`
   original mas é usado em `prog-bubblesort.ling` e `prog-factorial.ling`.
   Tratado simetricamente a `>`.

## Gramática

```
Prog       → MainC DefCl

MainC      → 'class' Id '{' 'public' 'static' 'void' 'main'
             '(' 'String' '[' ']' Id ')' '{' Cmds '}' '}'

DefCl      → 'class' Id DefClHead '{' DefVar DefMet '}' DefCl
           | λ

DefClHead  → 'extends' Id
           | λ

DefVar     → Type Id ';' DefVar
           | λ

DefMet     → 'public' Type Id '(' ArgsOpt ')'
             '{' DefVar Cmds 'return' Exp ';' '}' DefMet
           | λ

ArgsOpt    → Args
           | λ

Args       → Type Id ArgsRest
ArgsRest   → ',' Args
           | λ

Type       → 'int' TypeIntRest
           | 'boolean'
           | Id
TypeIntRest → '[' ']'
            | λ

Cmds       → Cmd Cmds
           | λ

Cmd        → '{' Cmds '}'
           | 'if' '(' Exp ')' Cmd 'else' Cmd
           | 'while' '(' Exp ')' Cmd
           | 'System' '.' 'out' '.' 'println' '(' Exp ')' ';'
           | Id CmdIdRest

CmdIdRest  → '=' Exp ';'
           | '[' Exp ']' '=' Exp ';'

Exp        → ExpAnd

ExpAnd     → ExpRel ExpAndRest
ExpAndRest → '&&' ExpRel ExpAndRest | λ

ExpRel     → ExpAdd ExpRelRest
ExpRelRest → '<' ExpAdd ExpRelRest
           | '>' ExpAdd ExpRelRest
           | λ

ExpAdd     → ExpMul ExpAddRest
ExpAddRest → '+' ExpMul ExpAddRest
           | '-' ExpMul ExpAddRest
           | λ

ExpMul     → ExpUnary ExpMulRest
ExpMulRest → '*' ExpUnary ExpMulRest | λ

ExpUnary   → '!' ExpUnary | ExpPostfix

ExpPostfix → ExpPrimary ExpPostfixRest
ExpPostfixRest → '[' Exp ']' ExpPostfixRest
              | '.' DotRest ExpPostfixRest
              | λ

ExpPrimary → 'new' NewRest
           | '(' Exp ')'
           | 'true' | 'false' | 'this'
           | Id | Number

NewRest    → Id '(' ')'
           | 'int' '[' Exp ']'

DotRest    → 'length'
           | Id '(' ListExpOpt ')'

ListExpOpt → Exp ListExpRest
           | λ

ListExpRest → ',' Exp ListExpRest
            | λ
```

## Notas sobre disambiguação por lookahead

- **`Cmd` vs `DefVar`** quando o primeiro token é `Id` (tipo customizado): precisa
  de lookahead-2 para decidir entre `Id Id ;` (DefVar) e `Id = ...` ou `Id [...] =` (Cmd).
- **`DotRest`** após `Exp '.'`: se próximo é `length`, vira `Exp.length`; se é `Id`,
  vira chamada de método `Exp.id(args)`.
- **Precedência de operadores**: a gramática transformada **não** preserva
  precedência aritmética (ex.: `a + b * c` parseia left-to-right como `(a+b)*c`).
  Como o trabalho exige apenas "código está sintaticamente correto", deixamos
  assim. Para AST com precedência correta, seria necessário separar em
  ExpAnd → ExpRel → ExpAdd → ExpMul → ExpUnary → ExpPrimary.
