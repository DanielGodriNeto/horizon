# Política de confiança para mods Java (proposta, não implementada)

O sidecar executa o código do mod em uma JVM separada, mas **sem sandbox além do processo**: o mod lê arquivos,
abre rede e roda processos com os direitos do usuário. Hoje só roda o mod nosso (`gemjava`). Para mod de terceiro:

1. **Lista de permitidos por hash.** Um mod só roda se o sha256 do jar estiver em `horizon-mods-java.allow`
   (uma linha: `sha256  nome  fonte`). Nome sozinho não vale; atualizar o mod exige reaprovar o hash.
2. **Origem.** Só entram jars baixados do Modrinth com sha512 conferido (ou compilados do código-fonte revisado).
   CurseForge e links soltos ficam de fora até haver verificação equivalente.
3. **Gate duplo.** Feature `foreign` na compilação **e** `HORIZON_JAVA_BRIDGE` no ambiente, como hoje. Nada liga por padrão.
4. **Perfil da JVM** (a implementar antes de aceitar terceiros): `--sun-misc-unsafe-memory-access=deny`,
   política que nega `java.net`, `ProcessBuilder` e escrita fora da pasta do mundo (SecurityManager já não existe no
   JDK 25: usar usuário Windows sem rede / Job Object + regra de firewall para o `java.exe` do sidecar).
5. **Mixins não rodam.** O shim ignora `mixins.json`; um mod que depende de mixin é recusado, não executado pela metade.
6. **Primeiros candidatos** (sem mixin, só API pública): `fabric-permissions-api`, `cloth-config`. Mods com centenas de
   mixins (Carpet, Lithium, Ledger) ficam fora de escopo.
7. **Revogação.** Remover a linha da lista desliga o mod no próximo boot; a ponte nunca reinicia sozinha um mod fora da lista.

## Sandbox (implementado em `sandbox/sandbox.ps1`, para medição, ainda não ligado ao jogo)
AppContainer sem capacidades (sem rede, nem loopback; sem acesso aos arquivos do usuário) + Job Object (kill-on-close, 1 processo, memória, timeout), env mínimo, HOME/tmp descartáveis, cwd vazio, tudo em pasta temporária removida ao final. A prova com um jar que tenta socket/leitura/escrita fora/spawn falha em todos. Não cobre: exploit de kernel, arquivos legíveis a todos, side channels, serviços ALPC, nem equivale a VM. Rodar mod de terceiro no jogo ainda exige este perfil mais a lista por hash.
