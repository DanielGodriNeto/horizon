# Protocolo do anel (P3), versão 1

Um arquivo mapeado em memória, criado pelo Rust (pai), mapeado também pelo Java (filho). Little-endian. Tamanho do arquivo = `64 + 2 * (128 + CAP)`, com `CAP = 1 << 20` por padrão (potência de 2).

| offset | campo |
|---|---|
| 0 | u32 magic `0x484D4A31` |
| 4 | u32 versão = 1 |
| 8 | u32 CAP |
| 16 | u32 `java_ready` (Java escreve 1 com volatile quando pronto) |
| 20 | u32 `shutdown` (Rust escreve 1; Java sai do laço) |
| 64 | anel A (Rust→Java): controle de 128 bytes |
| 64+128+CAP | anel B (Java→Rust): controle de 128 bytes |

Cada anel: `+0` u64 `head` (índice de leitura do consumidor, só cresce, em bytes), `+64` u64 `tail` (índice de escrita do produtor, só cresce), `+128` os CAP bytes de dados. `head` e `tail` ficam em linhas de cache separadas. Posição nos dados = índice mod CAP. Espaço livre = CAP - (tail - head).

Registro: u32 `len` + `len` bytes de payload, preenchido com zeros até múltiplo de 8 (o registro ocupa `8 * ceil((4+len)/8)` bytes). Um registro nunca atravessa o fim do buffer: se não couber até o fim, o produtor grava `len = 0xFFFFFFFF` (marcador de pulo) e continua em posição 0 (o consumidor ao ver o marcador avança até o fim). O produtor escreve o registro inteiro e só depois publica `tail` (release); o consumidor lê `tail` (acquire), consome registros e publica `head` (release).

Modo `echo`: o Java copia cada registro recebido do anel A para o anel B, sem alterar. Lote de k mensagens = k registros seguidos; o Rust publica `tail` uma vez por lote e espera k respostas.
Mensagens de controle: nenhuma na v1 (só `java_ready` e `shutdown` no cabeçalho).

## Adendo v1.1 (gerente, 2026-10-26, após o 1º bench)
Esclarecimentos: (1) os bytes do marcador de pulo e do fim do buffer contam no `tail`/`head` (o produtor avança `tail` por `pad + rec`); (2) o produtor de A (Rust) deve drenar o anel B enquanto espera espaço ou respostas, senão há deadlock; (3) o consumidor publica `head` depois de copiar o registro.

Achado do bench v1: com o Java ocioso mais de 20 µs ele dorme com `parkNanos(50us)`, e no Windows isso acorda só em ~1-1,5 ms (p50 do lote de 1000 = 1403 µs contra 8 µs do lote de 100, que não chega a dormir). O servidor real tem 50 ms entre ticks, então o Java estaria sempre dormindo: o número que importa é a latência de acordar.

Despertar por pipe (sem dependência nova): cabeçalho ganha `u32 sleeping` no offset 24 (Java escreve com volatile). O Java filho é lançado com `stdin` em pipe.
- Java, quando ocioso por mais de SPIN (padrão 50 µs): `sleeping = 1` (volatile/seq-cst), relê `tail` de A (volatile); se há dados, `sleeping = 0` e segue; senão bloqueia em `System.in.read()` (um byte; depois drene `System.in.available()`), e ao acordar `sleeping = 0`. EOF no stdin = pai morreu ou fechou: tratar como shutdown e sair com 0.
- Rust, depois de publicar `tail` de A (release): `fence(SeqCst)`; se `sleeping == 1`, escreve 1 byte no stdin do filho (e flush). Pode haver bytes extras: o Java os ignora.
- `shutdown = 1` também deve acordar: Rust escreve um byte depois de gravar `shutdown`.

## Mensagens do host (modo `Sidecar <arquivo> host <mod.jar>`)

Sobre os mesmos anéis; payload little-endian, primeiro byte = tipo. O sidecar carrega o mod (`horizon.LiveHost`
do shim, por reflexão), grava os registros em B **antes** de `java_ready = 1` e só então atende A.

| Tipo | Sentido | Corpo |
|---|---|---|
| 1 REGISTER_ITEM | Java -> Rust | u16 tamanho do id, id UTF-8 (`ns:path`), JSON de conteúdo do item |
| 2 BLOCK_BREAK | Rust -> Java | u64 seq, i32 x, i32 y, i32 z, id do bloco UTF-8 (resto) |
| 3 REPLY | Java -> Rust | u64 seq, mensagens que o mod mandou ao jogador (UTF-8, `\n` entre elas; vazio se nenhuma) |

Cada BLOCK_BREAK gera exatamente um REPLY com o mesmo seq (o callback `PlayerBlockBreakEvents.AFTER` do mod roda no meio).
