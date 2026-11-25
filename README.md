<p align="center">
  <img src="docs/banner.png" alt="Conversor DBF/DBC para Parquet — Banner Oficial" width="100%" style="border-radius: 10px; box-shadow: 0 8px 24px rgba(0,0,0,0.25);" />
</p>

<p align="center">
  <a href="docs/hex_sticker_hd.png" title="Clique para ampliar o Selo Hexagonal Oficial">
    <img src="docs/hex_sticker.png" alt="Selo Hexagonal Oficial - DBF/DBC para Parquet" width="160px" align="right" style="margin-left: 20px; margin-bottom: 20px;" />
  </a>
</p>

# Conversor de Bases da Vigilância — DBF / DBC para Parquet

<p align="left">
  <img src="https://img.shields.io/badge/Rust-1.80+-orange.svg?style=for-the-badge&logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/Apache_Parquet-ZSTD-blue.svg?style=for-the-badge&logo=apache" alt="Apache Parquet" />
  <img src="https://img.shields.io/badge/Plataforma-Linux%20%7C%20Windows-green.svg?style=for-the-badge" alt="Linux & Windows" />
  <img src="https://img.shields.io/badge/Vigilância_em_Saúde-SMS--Rio-red.svg?style=for-the-badge" alt="SMS Rio" />
  <img src="https://img.shields.io/badge/Versão-1.0.0-purple.svg?style=for-the-badge" alt="v1.0.0" />
  <a href="docs/hex_sticker_hd.png"><img src="https://img.shields.io/badge/Hex_Sticker-Disponível-informational.svg?style=for-the-badge&logo=r" alt="Hex Sticker" /></a>
</p>

> **Autor**: Eugênio Araújo — Centro de Inteligência Epidemiológica da SMS Rio (CIE / SVS / SMS-Rio)  
> Aplicativo nativo em **Rust** de ultra-alta performance com interface gráfica moderna e intuitiva para conversão de bases de dados epidemiológicas legadas (arquivos `.dbf` dBase III/IV, `.dbc` DATASUS e pacotes compactados `.zip`, `.rar`, `.7z`) para o formato colunar otimizado **Apache Parquet** com compressão **ZSTD**.

---

## 🎬 Demonstração Visual e Tutorial em Vídeo

<p align="center">
  <img src="docs/demonstracao.gif" alt="Demonstração do Conversor DBF para Parquet" width="850px" style="border-radius: 8px; box-shadow: 0 4px 12px rgba(0,0,0,0.15);" />
</p>

- 🎥 **Vídeo de Demonstração em Alta Resolução**: [docs/demonstracao.mp4](docs/demonstracao.mp4) (H.264 720p)
- 📖 **Roteiro Completo para Gravação e Apresentação (Cenas 00:00 - 05:00)**: [docs/ROTEIRO_TUTORIAL_VIDEO.md](docs/ROTEIRO_TUTORIAL_VIDEO.md)
- 🏷️ **Selo Hexagonal Oficial (Hex Sticker estilo pacotes R)**:
  - Imagem PNG em Alta Resolução (1200x1386): [docs/hex_sticker_hd.png](docs/hex_sticker_hd.png)
  - Gráfico Vetorial Escalável (SVG): [docs/hex_sticker.svg](docs/hex_sticker.svg)
- 🖼️ **Banner Oficial do Repositório (1280x420)**: [docs/banner.png](docs/banner.png) | [docs/banner_hd.png](docs/banner_hd.png)

---

## ⚡ Por que migrar de DBF para Parquet na Saúde Pública?

Bases de dados em formato dBase (`.dbf`) originadas nos sistemas do DATASUS e Ministério da Saúde foram projetadas há mais de 30 anos e apresentam limitações severas para análises modernas:
- **Tamanho Excessivo**: Registros ocupam largura fixa com bytes de preenchimento desnecessários.
- **Tipos Fracos**: A maioria das datas cruciais (`DTOBITO`, `DTNASC`, `DT_NOTIFIC`) é salva como texto (`YYYYMMDD` ou `DDMMYYYY`), forçando parsing manual e erros de timezone.
- **Consultas Lentas**: Softwares analíticos precisam carregar o arquivo inteiro sequencialmente.

Com o **Apache Parquet (ZSTD)**:
- **Redução de até 90%** no espaço em disco.
- **Tipagem Forte e Enriquecimento de Datas**: Datas salvas como texto no DBF são enriquecidas e convertidas para o formato nativo `Date32` (`YYYY-MM-DD`).
- **Consultas Instantâneas**: Suporte a projeção e filtro direto em ferramentas modernas como **R (arrow/tidyverse)**, **Python (Polars/Pandas)** e **Power BI**.

### 📊 Benchmark Real: Rust vs R (`microdatasus` / `foreign`) vs Python (`Polars` / `Pandas`)

Para uma comparação transparente e 100% reproduzível, o benchmark abaixo foi executado com a base pública oficial do DATASUS **`DORJ2022.dbc` / `DORJ2022.dbf`** (*SIM — Mortalidade Rio de Janeiro 2022*, **150.806 registros × 87 colunas**; **11,38 MB** em `.dbc` e **70,7 MB** em `.dbf`), medindo a **média de 3 execuções consecutivas** (tempo total ponta a ponta com escrita Parquet ZSTD) e o **pico de memória RAM** (Maximum RSS via `/usr/bin/time -v`).

> **💻 Configuração da Máquina de Teste**
> - **CPU**: Intel® Core™ i9-10900X @ 3.70 GHz (10 núcleos / 20 threads)
> - **Memória RAM**: 64 GB DDR4
> - **Sistema Operacional**: Linux Mint 21.3 (Kernel Linux 5.15.0 `x86_64`)
> - **Versões**: Rust (`--release` nativo), R `4.6.2`, Python `3.10.12`

#### 1. Conversão de **DBF → Parquet (ZSTD)** (`DORJ2022.dbf` — 70,7 MB)

| Ferramenta / Stack | Tempo Médio | Comparação (Tempo) | Pico de RAM | Parquet Gerado (ZSTD) | Enriquecimento de Datas (`Date32`) |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **🦀 Rust (`dbf-para-parquet`)** | **0,98 s** | **1,0x (Referência)** | **63,6 MB** | **5,06 MB** *(92,8% menor)* | ✅ Sim (automático) |
| **📊 R (`foreign::read.dbf` + `nanoparquet`)** | 2,25 s | 2,3x mais lento | 247,3 MB *(3,9x)* | 10,43 MB | ❌ Não (texto puro) |
| **🐍 Python (`dbfread` + `Polars`)** | 12,66 s | 12,9x mais lento | 2.622,0 MB *(41,2x)* | 4,91 MB | ❌ Não (texto puro) |
| **🐍 Python (`dbfread` + `Pandas` + `PyArrow`)** | 16,95 s | 17,3x mais lento | 2.514,8 MB *(39,5x)* | 5,26 MB | ❌ Não (texto puro) |

#### 2. Conversão de **DBC → Parquet (ZSTD)** (`DORJ2022.dbc` — 11,38 MB)

No R, a simulação reproduz fielmente a leitura de `.dbc` do pacote **[`microdatasus` v3.0.0](https://github.com/rfsaldanha/microdatasus)** (de Raphael Saldanha), utilizando o mesmo descompressor C (`dbc2dbf.c` + `blast.c`) seguido de `foreign::read.dbf` e conversão para `tibble`, gravando em Parquet ZSTD com `nanoparquet`.

| Ferramenta / Stack | Tempo Médio | Pico de RAM | Parquet Gerado (ZSTD) | Enriquecimento de Datas (`Date32`) |
| :--- | :---: | :---: | :---: | :---: |
| **🦀 Rust (`dbf-para-parquet`)** | **4,49 s** | **63,1 MB** | **5,06 MB** | ✅ Sim (automático) |
| **📊 R (`microdatasus::read_dbc` + `nanoparquet`)** | **3,08 s** | 301,3 MB *(4,8x)* | 10,43 MB *(2,1x maior)* | ❌ Não (texto puro) |
| **🐍 Python (`datasus-dbc` + `dbfread` + `Polars`)** | 16,09 s | 2.623,3 MB *(41,6x)* | 4,91 MB | ❌ Não (texto puro) |
| **🐍 Python (`datasus-dbc` + `dbfread` + `Pandas` + `PyArrow`)** | 20,91 s | 2.515,2 MB *(39,9x)* | 5,26 MB | ❌ Não (texto puro) |

#### Principais Conclusões do Benchmark
- **Uso Constante e Mínimo de Memória (Streaming)**: o **Rust (`dbf-para-parquet`)** manteve o pico em **~63 MB de RAM** tanto em `.dbf` quanto em `.dbc` — **~4x a 5x menos RAM que o R** e **~40x menos RAM que o Python** (`~2,5 GB` de RAM). Isso permite converter bases muito grandes ou vários anos seguidos mesmo em computadores com pouca memória.
- **Liderança Absoluta em DBF**: converteu 70,7 MB (150 mil linhas) em **menos de 1 segundo** (**2,3x mais rápido que R**, **12,9x mais rápido que Python+Polars** e **17,3x mais rápido que Python+Pandas**), reduzindo o arquivo em **92,8%** (de 70,7 MB para 5,06 MB).
- **DBC com Valor Agregado**: embora o descompressor em C do `microdatasus` conclua a leitura pura do `.dbc` ligeiramente mais rápido (3,08 s vs 4,49 s), o **Rust (`dbf-para-parquet`)** utiliza **4,8x menos RAM**, gera um Parquet **2x menor** (5,06 MB vs 10,43 MB) e entrega a base **já tipada com enriquecimento automático de datas (`Date32`)**, limpeza de strings (`TRIM`/`NULL`), identificação do sistema e auditoria LGPD — sem precisar instalar R, compiladores ou dependências externas.

---

## 🚀 Principais Funcionalidades

### 1. Dicionário Oficial e Reconhecimento Automático de 6 Sistemas
O conversor analisa as colunas do cabeçalho DBF antes do processamento e compara com os esquemas oficiais do DATASUS:
- **SIM** (*Sistema de Informações sobre Mortalidade*): 147 colunas oficiais (`NUMERODO`, `DTOBITO`, `CAUSABAS`, `ATESTANTE`, `LINHAA`, etc.)
- **SINASC** (*Sistema de Informações sobre Nascidos Vivos*): 106 colunas oficiais (`NUMERODN`, `DTNASC`, `HORANASC`, `APGAR1`, `PARTO`, etc.)
- **SIVEP SRAG** (*Hospitalizados por Síndrome Respiratória Aguda Grave*): 221 colunas oficiais (`NU_NOTIFIC`, `DT_NOTIFIC`, `DT_SIN_PRI`, `VACINA_COV`, `DT_INTERNA`, etc.)
- **SIVEP SG** (*Sentinela de Síndrome Gripal*): 160 colunas oficiais (`NUM_REGIST`, `DT_PREENC`, `DT_PRISINT`, `SIND_DOWN`, etc.)
- **DENGON** (*Dengue e Chikungunya Online / Arboviroses*): 159 colunas oficiais (`ID_AGRAVO`, `DOR_RETRO`, `DT_CHIK_S1`, `DT_NS1`, `DT_SORO`, etc.)
- **SINAN NET** (*Sistema de Informação de Agravos de Notificação*): mais de 2.100 colunas oficiais catalogadas cobrindo todas as 60+ tabelas de agravos (`ACBIONET`, `ANIMPNET`, `TUBENET`, `VIOLENET`, `AIDSANET`, `HEPANET`, `SIFGENET`, etc.) e tabelas de apoio territorial (`BAIRRNET`, `MUNICNET`, etc.)

O diagnóstico em tempo real calcula a porcentagem exata de correspondência e destaca o sistema identificado.

### 2. Codificação de Caracteres (Encoding)
- **Padrão automático**: utiliza **`ISO-8859-1` / `Windows-1252`**, garantindo a leitura correta de acentos e cedilhas (`ç`, `ã`, `é`) na grande maioria das bases de saúde.
- **Opções manuais disponíveis**: caso possua uma base mais antiga ou exportada em outro padrão, você pode escolher diretamente na interface (ou no terminal) entre **`Automático (ISO-8859-1)`**, **`ISO-8859-1`**, **`CP850` (MS-DOS antigo)** e **`UTF-8`**.

### 3. Conformidade LGPD e Auditoria de Dados Pessoais
- **Base Identificada (Com dados pessoais de pacientes)**: Detecta campos nominais do paciente ou familiares (`NOME`, `NOMEMAE`, `NOMEPAI`, `NM_PACIENT`, `NM_MAE_PAC`, `NU_CPF`, `NU_CNS`, `ENDRES`, `CEPRES`, etc.).
- **Base Anonimizada (Sem dados pessoais)**: Identifica bases públicas onde colunas nominais foram removidas.
- **Isenção de Registro Profissional**: Nomes e registros de médicos e atestantes (`CRM`, `MEDICO`, `COREN`) são dados públicos de identificação profissional e **não** classificam o paciente individualmente.

### 4. Enriquecimento dos Campos de Data e Tratamento de Tipos
- **Converter string para Date32 (Padrão Recomendado)**:
  - **Enriquecimento automático de datas**: identifica campos de data salvos originalmente como texto no DBF/DBC (`DTOBITO`, `DTNASC`, `DT_NOTIFIC`, `DT_SIN_PRI`, `DT_INTERNA`, etc.) nos formatos `DDMMYYYY`, `YYYYMMDD`, `DD/MM/YYYY` ou `YYYY-MM-DD` e os converte diretamente para o tipo nativo **Date32** (`YYYY-MM-DD`) do Parquet, permitindo filtros por período, ordenação cronológica e cálculos de intervalo imediatamente no R, Python ou Power BI.
  - **Limpeza de dados**: remove espaços excedentes nas extremidades (TRIM) e converte datas inválidas (`00000000`, `________`) e valores em branco em `NULL` reais.
- **Manter datas como TEXTO (Modo DBF Estrito)**:
  - Respeita rigorosamente os tipos declarados no cabeçalho binário DBF, mantendo campos de data declarados como texto (`C`) sem conversão para `Date32`.

### 5. Descompressão Nativa de Arquivos DBC do DATASUS (FTP DISSEMIN)

> [!IMPORTANT]
> **Fim da Dependência do TabWin e read.dbc**: No repositório público oficial do DATASUS (`ftp://ftp.datasus.gov.br/dissemin/publicos/`), os microdados abertos para pesquisadores são distribuídos no formato proprietário **`.dbc`**. 
> O **Conversor de Bases da Vigilância para Parquet** agora incorpora suporte nativo e transparente ao formato `.dbc`:
> - **Arrastar e Soltar Direto**: Basta soltar qualquer arquivo `.dbc` na janela do programa.
> - **Zero Configuração**: Descompacta em memória/temporário na velocidade nativa do Rust, detecta o sistema epidemiológico, audita os campos como **Base Anonimizada** e grava diretamente em Apache Parquet ZSTD.
> - **Suporte a Pacotes Compactados**: Também aceita pacotes `.zip`, `.rar` e `.7z` contendo arquivos DBF internos, com descarte e faxina automática de temporários ao final do processamento.

### 6. Junção de Arquivos (Merge) ou Conversão Individual
- **Conversão Individual**: Cada DBF gera seu `.parquet` correspondente (com opção de escolher nome de saída quando processado arquivo avulso).
- **Junção de Arquivos (Merge)**: Combina múltiplos arquivos ou anos do mesmo sistema em um único arquivo `.parquet` consolidado.

---

## 📥 Download dos Executáveis (Sem Instalação)

Não é necessário instalar nada! O aplicativo é um executável nativo único e independente:

| Plataforma | Arquivo | Como Usar |
| :--- | :--- | :--- |
| **Windows (10 / 11)** | [**`dbf-para-parquet-windows.exe`**](https://github.com/eugeniosinan/DBF-para-Parquet/releases/latest/download/dbf-para-parquet-windows.exe) | Baixe e dê duplo clique para abrir. Não requer permissão de administrador. |
| **Windows (.zip)** | [**`dbf-para-parquet-windows.zip`**](https://github.com/eugeniosinan/DBF-para-Parquet/releases/latest/download/dbf-para-parquet-windows.zip) | Versão compactada portátil (14 MB). |
| **Linux (64-bit)** | [**`dbf-para-parquet-linux`**](https://github.com/eugeniosinan/DBF-para-Parquet/releases/latest/download/dbf-para-parquet-linux) | Dê permissão de execução (`chmod +x`) e execute no terminal ou com duplo clique. |

---

## 🖥️ Como Usar a Interface Gráfica (4 Passos)

1. **Passo 1 — Seleção de Arquivos ou Pastas**:
   - Clique em **"Adicionar Arquivos"** ou **"Adicionar Pasta"**, ou simplesmente arraste e solte arquivos (`.dbf`, `.zip`, `.rar`, `.7z`) na janela do aplicativo.
2. **Passo 2 — Diagnóstico e Padronização**:
   - O aplicativo exibe a tabela de arquivos identificados com o nome do sistema (ex: `SIM (100% match)`), status de dados pessoais (LGPD) e encoding sugerido.
3. **Passo 3 — Opções de Conversão**:
   - Escolha o modo de saída: **Individual** ou **Juntar em um único Parquet (Merge)**.
   - Escolha o tratamento de tipos: **Padrão Saúde** (recomendado para datas automáticas) ou **Padrão DBF Estrito**.
   - Defina a pasta ou arquivo de destino.
4. **Passo 4 — Execução**:
   - Clique em **"Iniciar Conversão"**. Acompanhe a barra de progresso em tempo real e visualize o relatório detalhado ao término.

---

## ⌨️ Modo Linha de Comando (CLI) para Automação

O aplicativo inclui modo terminal completo para integração com pipelines de dados (Airflow, Cron, rotinas agendadas, scripts Bash ou PowerShell):

### 💡 Exemplos Rápidos de 1 Linha (Copie e Cole)

#### 1. Inspecionar e Diagnosticar um Arquivo (sem converter):
Exibe o sistema identificado (SIM, SINASC, SIVEP, etc.), total de linhas, percentual de correspondência do schema, colunas de data, encoding e auditoria LGPD:
```bash
# No Linux:
./dbf-para-parquet-linux --inspect "DORJ2025.dbc"

# No Windows (Prompt de Comando ou PowerShell):
dbf-para-parquet-windows.exe --inspect "DORJ2025.dbc"
```

#### 2. Converter um Arquivo Avulso (DBF, DBC, ZIP, RAR ou 7Z):
Gera o arquivo `.parquet` colunar com compressão ZSTD e tipagem automática:
```bash
# No Linux:
./dbf-para-parquet-linux --cli "DORJ2025.dbc" "DORJ2025.parquet"

# No Windows:
dbf-para-parquet-windows.exe --cli "DORJ2025.dbc" "DORJ2025.parquet"

# Consolidar múltiplos anos em um único Parquet (--merge):
./dbf-para-parquet-linux --cli DORJ2020.dbc DORJ2021.dbc DORJ2022.dbc --merge -o "DO_RJ_Serie.parquet"

# Converter todos os arquivos de uma pasta para outro diretório (--out-dir):
./dbf-para-parquet-linux --cli /dados/sivep/ --out-dir /dados/parquet/

# Forçar encoding específico (se necessário):
./dbf-para-parquet-linux --cli "entrada.dbf" -o "saida.parquet" --encoding CP850

# Usar modo DBF estrito em vez do Padrão Saúde de datas:
./dbf-para-parquet-linux --cli "entrada.dbf" -o "saida.parquet" --strict
```

#### Tabela Completa de Paridade: Interface Gráfica vs Linha de Comando (CLI)

| Funcionalidade na Interface Gráfica | Equivalente na Linha de Comando (CLI) | Descrição |
| :--- | :--- | :--- |
| **Arrastar Arquivos ou Pastas** | Passar arquivos ou diretórios como argumentos | Aceita caminhos de arquivos (`.dbf`, `.dbc`, `.zip`, `.rar`, `.7z`) ou pastas inteiras |
| **Modo de Saída: Individual** | Padrão (ou `--out-dir <PASTA>`) | Gera um arquivo Parquet individual para cada DBF/DBC |
| **Modo de Saída: Unificado (Merge)** | `-m`, `--merge -o <ARQUIVO.parquet>` | Concatena/consolida múltiplos arquivos em uma única tabela Parquet |
| **Converter string para Date32** | Padrão ativado | Enriquecimento de datas: converte strings `DDMMAAAA` / `YYYYMMDD` em datas nativas (`Date32`) |
| **Manter datas como TEXTO** | `--strict` | Respeita estritamente os tipos originais do DBF, mantendo datas em texto |
| **Codificação (Encoding)** | `--encoding <auto\|iso-8859-1\|cp850\|utf-8>` | Padrão `ISO-8859-1`, com opções manuais para `CP850` e `UTF-8` |
| **Aba Pré-Processamento / Auditoria LGPD** | `--inspect <ARQUIVO>` | Realiza o diagnóstico completo, contagem de linhas e auditoria LGPD sem converter |
| **Ajuda Integrada** | `-h`, `--help` | Exibe a documentação de uso e exemplos diretamente no terminal |

> [!TIP]
> **Atenção aos caminhos com espaços**: Se a pasta ou o arquivo tiver espaços (como `"/home/usuario/Bases de Exemplo/arquivo.zip"`), **sempre envolva o caminho entre aspas duplas `"..."`**.

---

### 🔧 Como Usar o Comando Diretamente como `dbf-para-parquet` (Sem `./` no Linux)

No Linux, você pode instalar o comando no seu diretório de binários para poder digitar apenas `dbf-para-parquet` de qualquer pasta do terminal:

```bash
# Para o seu usuário local (não precisa de senha de administrador/sudo):
mkdir -p ~/.local/bin
cp dist/dbf-para-parquet-linux ~/.local/bin/dbf-para-parquet
chmod +x ~/.local/bin/dbf-para-parquet

# Ou para todos os usuários do sistema:
sudo cp dist/dbf-para-parquet-linux /usr/local/bin/dbf-para-parquet
```

Feito isso, você poderá rodar simplesmente:
```bash
dbf-para-parquet --inspect "caminho/do/arquivo.zip"
dbf-para-parquet --cli "caminho/do/arquivo.dbc" "saida.parquet"
```

### Automação em Lote (Diretórios Inteiros e Scripts)

Não é necessário criar loops manuais (`for` no bash ou `ForEach-Object` no PowerShell)! O conversor aceita pastas inteiras nativamente:

#### 1. Converter Todos os Arquivos de uma Pasta (Modo Individual)
Varre recursivamente a pasta de entrada processando `.dbf`, `.dbc`, `.zip`, `.rar` e `.7z`:
```bash
# No Linux:
dbf-para-parquet --cli "/dados/vigilancia/entradas/" --out-dir "/dados/vigilancia/parquet/"

# No Windows:
dbf-para-parquet-windows.exe --cli "C:\BasesVigilancia\Originais" --out-dir "C:\BasesVigilancia\Parquet"
```

#### 2. Juntar Arquivos com a Mesma Estrutura em um Único Parquet (Modo Unificado / Merge)
Lê todos os arquivos da pasta (ou passados na lista) e **consolida tudo em um único arquivo Parquet ultra-compactado**:
```bash
# No Linux (ex: juntar todos os anos do SIM ou SINAN de uma pasta):
dbf-para-parquet --cli "/dados/sim_anos/" --merge -o "/dados/parquet/SIM_Consolidado_2010_2024.parquet"

# No Windows:
dbf-para-parquet-windows.exe --cli "C:\Bases\SIM" --merge -o "C:\Bases\SIM_Consolidado.parquet"
```

#### 3. Integração em Python (via subprocess)
```python
import subprocess

# Converter pasta inteira para outra pasta em 1 linha:
subprocess.run([
    "dbf-para-parquet", "--cli",
    "/dados/vigilancia/entradas/",
    "--out-dir", "/dados/vigilancia/parquet/"
], check=True)

# Consolidar todos os arquivos da pasta em um único Parquet:
subprocess.run([
    "dbf-para-parquet", "--cli",
    "/dados/vigilancia/sim/",
    "--merge",
    "-o", "/dados/vigilancia/SIM_Consolidado.parquet"
], check=True)
```

---

## 📊 Como Abrir e Visualizar Arquivos Parquet (Excel, Power BI e Visualizadores)

Muitos profissionais da saúde e vigilância estão habituados com o Microsoft Excel. Embora o Parquet não abra com duplo clique no Excel antigo, existem diversas formas extremamente fáceis e acessíveis para visualizá-lo:

### 1. Microsoft Excel via Power Query (Nativo no Microsoft 365 / Excel 2021)
O Excel moderno já inclui o conector nativo de arquivos Parquet através do **Power Query**, sem necessidade de instalar extensões adicionais:
1. Abra uma pasta de trabalho em branco no Excel.
2. Acesse a guia **Dados** no menu superior.
3. Clique em **Obter Dados** ➔ **Do Arquivo** ➔ **Do Parquet**.
4. Selecione o arquivo `.parquet` gerado pelo conversor.
5. Uma janela de navegação exibirá a pré-visualização tabular dos dados.
6. Clique em **Transformar Dados** (caso queira selecionar colunas específicas ou filtrar apenas seu município/ano) ou clique diretamente em **Carregar**.

> [!TIP]
> **Vantagem sobre o DBF tradicional**: O Excel padrão é limitado a 1.048.576 linhas. Ao usar o Power Query com Parquet, você pode filtrar e agregar bases com **milhões de linhas** (como anos consolidados de nascimentos ou óbitos) sem estourar o limite de memória do computador!

### 2. Power BI Desktop (Visualização e Criação de Painéis)
1. No Power BI, clique em **Obter Dados** ➔ **Mais...** ➔ **Parquet**.
2. Aponte para o arquivo `.parquet`. Todas as datas (`Date32`) e colunas de texto serão importadas com os tipos já perfeitamente ajustados, prontas para relacionamentos e gráficos.

### 3. Programas Visuais Gratuitos com Cara de Planilha ("Duplo Clique")

Para quem deseja apenas dar duplo clique e abrir a tabela em formato de grade com filtros e buscas:

- **[ParquetViewer](https://github.com/mukunku/ParquetViewer)** (Windows - Gratuito e Portátil):
  - Executável levíssimo sem necessidade de instalação.
  - Ao associar arquivos `.parquet` a ele, o arquivo abre em uma grade visual idêntica ao Excel.
  - Possui busca textual instantânea, ordenação, paginação de grandes volumes e botão **"Export to Excel"** para gerar planilhas `.xlsx`.
- **[Tad Data Viewer](https://www.tadviewer.com/)** (Windows, Linux e macOS - Gratuito):
  - Interface desktop moderna desenhada especificamente para arquivos Parquet e CSV de grande porte.
  - Permite criar tabelas dinâmicas (*pivot tables*), histogramas de distribuição e filtros multidimensionais em milissegundos.
- **[DBeaver Community](https://dbeaver.io/)** (Windows e Linux):
  - Gerenciador universal de dados com suporte para navegar em pastas de arquivos Parquet via interface visual.

### 4. Como Exportar do Parquet para Excel (`.xlsx`) ou CSV (Excel PT-BR)

Se for necessário fornecer uma planilha para quem utiliza versões legadas do Excel:

#### Exportar para CSV formatado para Excel em Português (separador `;`) ou `.xlsx` via Python:
```python
import polars as pl

df = pl.read_parquet("DORJ2025.parquet")

# Salvar como planilha .xlsx nativa do Excel:
df.write_excel("DORJ2025.xlsx")

# Ou salvar como CSV com separador ';' para Excel em Português:
df.write_csv("DORJ2025_excel.csv", separator=";")
```

---

## 📈 Análise de Dados e Scripts (R e Python)

### 1. No R (`arrow` e `tidyverse`)
Pesquisadores e analistas em saúde pública podem consumir os arquivos Parquet no R com velocidade extrema:

```r
# Usando o pacote arrow:
library(arrow)
library(dplyr)

# Leitura completa da base tipada:
dados <- read_parquet("DORJ2025.parquet")

# Ou leitura sob demanda (lazy) sem sobrecarregar a memória RAM:
dataset <- open_dataset("DORJ2025.parquet")

resultado <- dataset %>%
  filter(LOCOCOR == "1") %>% # apenas óbitos hospitalares
  group_by(CAUSABAS) %>%
  summarise(total = n()) %>%
  arrange(desc(total)) %>%
  collect()

print(head(resultado, 10))
```

### 2. No Python (`Polars` e `Pandas`)

#### Com **Polars** (Recomendado pela velocidade ultrarrápida):
```python
import polars as pl

# Carregamento imediato com tipagem forte:
df = pl.read_parquet("DORJ2025.parquet")
print(df.schema)

# Agrupamento e agregação:
top_causas = (
    df.filter(pl.col("CAUSABAS").is_not_null())
    .group_by("CAUSABAS")
    .agg(pl.count().alias("total"))
    .sort("total", descending=True)
    .head(10)
)
print(top_causas)
```

#### Com **Pandas**:
```python
import pandas as pd

df = pd.read_parquet("DORJ2025.parquet")
print(f"Total de registros: {len(df)}")
```

---

## 🛠️ Como Compilar a Partir do Código Fonte

### Pré-requisitos
- [Rust & Cargo](https://rustup.rs/) (versão 1.80+)
- No Linux: bibliotecas de desenvolvimento do X11 / Wayland e gráficos (`libasound2-dev`, `libfontconfig1-dev`, `libx11-dev`).

### Compilar para Linux:
```bash
cargo build --release
# O executável será gerado em target/release/dbf-para-parquet
```

### Compilação Cruzada para Windows (usando Cross e Podman/Docker):
```bash
./build_windows.sh
# O executável será gerado em dist/dbf-para-parquet-windows.exe
```

### Executar os Testes Automatizados:
```bash
cargo test
```

---

## 🏛️ Créditos e Autoria

- **Autor**: Eugênio Araújo
- **Lotação**: Centro de Inteligência Epidemiológica da SMS Rio (CIE / SVS / SMS-Rio)
- **Tecnologias**: Rust, Apache Arrow, Apache Parquet (ZSTD), eframe / egui, wgpu.

---

## 📄 Licença

Este projeto é disponibilizado sob a licença **MIT** — livre para uso, modificação e distribuição por órgãos públicos, pesquisadores e pela comunidade de dados em saúde.
