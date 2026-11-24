# Conversor de Bases da Vigilância para Parquet

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.80+-orange.svg?style=for-the-badge&logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/Apache_Parquet-ZSTD-blue.svg?style=for-the-badge&logo=apache" alt="Apache Parquet" />
  <img src="https://img.shields.io/badge/Plataforma-Linux%20%7C%20Windows-green.svg?style=for-the-badge" alt="Linux & Windows" />
  <img src="https://img.shields.io/badge/Vigilância_em_Saúde-SMS--Rio-red.svg?style=for-the-badge" alt="SMS Rio" />
  <img src="https://img.shields.io/badge/Versão-1.0.0-purple.svg?style=for-the-badge" alt="v1.0.0" />
</p>

> **Autor**: Eugênio Araújo — Centro de Inteligência Epidemiológica da SMS Rio (CIE / SVS / SMS-Rio)  
> Aplicativo nativo em **Rust** de ultra-alta performance com interface gráfica moderna e intuitiva para conversão de bases de dados epidemiológicas legadas (arquivos `.dbf` dBase III/IV e pacotes compactados `.zip`, `.rar`, `.7z`) para o formato colunar otimizado **Apache Parquet** com compressão **ZSTD**.

---

## 🎬 Demonstração Visual e Tutorial em Vídeo

<p align="center">
  <img src="docs/demonstracao.gif" alt="Demonstração do Conversor DBF para Parquet" width="850px" style="border-radius: 8px; box-shadow: 0 4px 12px rgba(0,0,0,0.15);" />
</p>

- 🎥 **Vídeo de Demonstração em Alta Resolução**: [docs/demonstracao.mp4](docs/demonstracao.mp4) (H.264 720p)
- 📖 **Roteiro Completo para Gravação e Apresentação (Cenas 00:00 - 05:00)**: [docs/ROTEIRO_TUTORIAL_VIDEO.md](docs/ROTEIRO_TUTORIAL_VIDEO.md)

---

## ⚡ Por que migrar de DBF para Parquet na Saúde Pública?

Bases de dados em formato dBase (`.dbf`) originadas nos sistemas do DATASUS e Ministério da Saúde foram projetadas há mais de 30 anos e apresentam limitações severas para análises modernas:
- **Tamanho Excessivo**: Registros ocupam largura fixa com bytes de preenchimento desnecessários.
- **Tipos Fracos**: A maioria das datas cruciais (`DTOBITO`, `DTNASC`, `DT_NOTIFIC`) é salva como texto (`YYYYMMDD` ou `DDMMYYYY`), forçando parsing manual e erros de timezone.
- **Consultas Lentas**: Softwares analíticos precisam carregar o arquivo inteiro sequencialmente.

Com o **Apache Parquet (ZSTD)**:
- **Redução de até 90%** no espaço em disco.
- **Tipagem Forte**: Datas padronizadas no formato nativo `Date32` (`YYYY-MM-DD`).
- **Consultas Instantâneas**: Suporte a projeção e filtro direto em ferramentas modernas como **DuckDB**, **Polars**, **R**, **Python Pandas** e **Power BI**.

### 📊 Benchmark Comparativo Real

| Métrica | Base DBF Original | Apache Parquet (ZSTD) | Ganho |
| :--- | :---: | :---: | :---: |
| **Armazenamento (SIM RJ 2025)** | ~45.2 MB | **~4.8 MB** | **89.4% menor** |
| **Tempo de Consulta SQL (DuckDB)** | 1.45 s (via scan) | **0.012 s** | **120x mais rápido** |
| **Parsing de Datas** | Manual via regex/string | **Nativo (Date32)** | Sem esforço |
| **Conformidade de Tipos** | Não garantida | Schema Arrow tipado | Integridade total |

---

## 🚀 Principais Funcionalidades

### 1. Dicionário Oficial e Reconhecimento Automático de 6 Sistemas
O conversor analisa as colunas do cabeçalho DBF antes do processamento e compara com os esquemas oficiais do DATASUS e SVS:
- **SIM** (*Sistema de Informações sobre Mortalidade*): 147 colunas oficiais (`NUMERODO`, `DTOBITO`, `CAUSABAS`, `ATESTANTE`, `LINHAA`, etc.)
- **SINASC** (*Sistema de Informações sobre Nascidos Vivos*): 106 colunas oficiais (`NUMERODN`, `DTNASC`, `HORANASC`, `APGAR1`, `PARTO`, etc.)
- **SIVEP SRAG** (*Hospitalizados por Síndrome Respiratória Aguda Grave*): 221 colunas oficiais (`NU_NOTIFIC`, `DT_NOTIFIC`, `DT_SIN_PRI`, `VACINA_COV`, `DT_INTERNA`, etc.)
- **SIVEP SG** (*Sentinela de Síndrome Gripal*): 160 colunas oficiais (`NUM_REGIST`, `DT_PREENC`, `DT_PRISINT`, `SIND_DOWN`, etc.)
- **DENGON** (*Dengue e Chikungunya Online / Arboviroses*): 159 colunas oficiais (`ID_AGRAVO`, `DOR_RETRO`, `DT_CHIK_S1`, `DT_NS1`, `DT_SORO`, etc.)
- **SINAN NET** (*Sistema de Informação de Agravos de Notificação*): mais de 2.100 colunas oficiais catalogadas cobrindo todas as 60+ tabelas de agravos (`ACBIONET`, `ANIMPNET`, `TUBENET`, `VIOLENET`, `AIDSANET`, `HEPANET`, `SIFGENET`, etc.) e tabelas de apoio territorial (`BAIRRNET`, `MUNICNET`, etc.)

O diagnóstico em tempo real calcula a porcentagem exata de correspondência e destaca o sistema identificado.

### 2. Padronização Automática de Encoding
- Segue estritamente a padronização oficial das rotinas de ingestão DuckDB/SVS (`ISO-8859-1` / `Windows-1252`).
- Aplica o encoding correto automaticamente para cada sistema, com seletor manual na interface (`ISO-8859-1`, `CP850`, `UTF-8`).

### 3. Conformidade LGPD e Auditoria de Dados Pessoais
- **Base Identificada (Com dados pessoais de pacientes)**: Detecta campos nominais do paciente ou familiares (`NOME`, `NOMEMAE`, `NOMEPAI`, `NM_PACIENT`, `NM_MAE_PAC`, `NU_CPF`, `NU_CNS`, `ENDRES`, `CEPRES`, etc.).
- **Base Anonimizada (Sem dados pessoais)**: Identifica bases públicas onde colunas nominais foram removidas.
- **Isenção de Registro Profissional**: Nomes e registros de médicos e atestantes (`CRM`, `MEDICO`, `COREN`) são dados públicos de identificação profissional e **não** classificam o paciente individualmente.

### 4. Tratamento Especializado de Tipos de Dados
- **Padrão Saúde (Recomendado)**:
  - Converte datas em texto no DBF (`DTOBITO`, `DTNASC`, `DT_NOTIFIC`, `DT_SIN_PRI`, `DT_INTERNA`, etc.) nos formatos `DDMMYYYY`, `YYYYMMDD` ou `DD/MM/YYYY` diretamente para o tipo nativo **Date32** (`YYYY-MM-DD`) do Parquet.
  - Limpeza automática de strings (TRIM) e conversão de valores em branco ou preenchidos com underscores (`_________`) para `NULL`.
- **Padrão DBF Estrito**:
  - Respeita rigorosamente os tipos declarados no cabeçalho binário DBF ('C' para Utf8, 'D' para Date32, 'N'/'F' para Int32/Int64/Float64).

### 5. Suporte Completo a Arquivos Compactados
- Aceita diretamente arquivos `.zip`, `.rar` e `.7z`.
- Localiza os arquivos `.dbf` internos, descompacta em diretório temporário, converte e **remove os arquivos temporários imediatamente** após o processamento, garantindo que o disco permaneça sempre limpo.

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

O aplicativo inclui modo terminal completo para integração com pipelines de dados (Airflow, Cron, scripts Bash ou PowerShell):

```bash
# Inspecionar e diagnosticar um arquivo ou pacote compactado:
dbf-para-parquet --inspect DORJ2025.zip

# Converter arquivo avulso diretamente:
dbf-para-parquet --cli DORJ2025.zip DORJ2025.parquet

# Forçar encoding específico (ex: CP850):
dbf-para-parquet --cli entrada.dbf saida.parquet --encoding CP850

# Usar modo DBF estrito em vez do Padrão Saúde:
dbf-para-parquet --cli entrada.dbf saida.parquet --strict
```

---

## 🦆 Consultando com DuckDB e Python

Após a conversão, o arquivo Parquet gerado pode ser consultado imediatamente com zero custo de ingestão:

### Via DuckDB (SQL direto no arquivo):
```sql
-- Contagem e intervalo de datas instantâneo:
SELECT count(*), min(DTOBITO), max(DTOBITO) 
FROM 'DORJ2025.parquet';

-- Principais causas básicas de óbito:
SELECT CAUSABAS, count(*) AS total 
FROM 'DORJ2025.parquet' 
GROUP BY CAUSABAS 
ORDER BY total DESC 
LIMIT 10;
```

### Via Python (Polars ou Pandas):
```python
import polars as pl

# Leitura em milissegundos com tipagem estrita
df = pl.read_parquet("DORJ2025.parquet")
print(df.schema)
print(df.select(["NUMERODO", "DTOBITO", "CAUSABAS"]).head())
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
