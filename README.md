# Ateliê Livre

Reimplementação livre, em Rust, de dois programas infantis de desenho e escrita
dos anos 1990 (Microsoft Fine Artist e Microsoft Creative Writer), para que
crianças possam usá-los em sistemas atuais, com tela ampliada e som funcionando.

O Ateliê Livre não contém nenhum arquivo original: ele lê os dados de uma cópia
que você já possui.

## Estado

Subprojeto 1 (leitura dos formatos de dados) concluído: todos os arquivos da
versão Windows 3.1 em português são lidos, e as imagens e os sons são
extraídos. A documentação dos formatos está em [`docs/formats/`](docs/formats/).
Os próximos passos (engine, Fine Artist, Creative Writer) estão descritos em
[`docs/superpowers/specs/`](docs/superpowers/specs/).

## Uso

É preciso ter o Rust (1.85 ou mais novo) e a pasta `MSKIDS` de uma instalação
do Fine Artist e/ou do Creative Writer, copiada para o seu computador.

```sh
cargo build --release

# inventário de um arquivo (executável, .PIC, documento, biblioteca, WAV)
target/release/atelie-tools info caminho/MSKIDS/ARTIST.EXE

# extrai imagens (PNG), sons (WAV), recursos e documentos para uma pasta
target/release/atelie-tools extract caminho/MSKIDS saida
```

O `extract` gera `saida/index.html`, uma folha de contato com todas as imagens
e os sons, e `saida/report.json`, com o que foi processado, o que falhou e o
que não foi encontrado. Os nomes dos arquivos podem estar em qualquer
combinação de maiúsculas e minúsculas, e os documentos são reconhecidos pelo
conteúdo, não pela extensão, em qualquer subpasta (inclusive `USERS/`). Códigos de saída: 0 sucesso, 1 houve
falhas, 2 a pasta não parece uma instalação compatível.

## Desenvolvimento

```sh
export ATELIE_DATA=caminho/MSKIDS   # habilita os testes com os dados reais
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Sem `ATELIE_DATA`, os testes com dados reais são pulados e só os testes com
dados sintéticos rodam. Os arquivos originais nunca entram no repositório.

## Licença

GPL-3.0-or-later (ver `LICENSE`).

Fine Artist, Creative Writer e demais nomes citados são marcas da Microsoft
Corporation; este projeto não tem vínculo com a Microsoft.
