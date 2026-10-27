# RESULTADO P3 - duas passadas completas

Passada = execução sem gap (lotes 1..10000) + gap 0, 1000 e 50000 µs (lotes 1..1000, pipe incluso). Gap 50000: 500 iterações.

---

## Passada 1

### default


- Plataforma: Windows, 16 threads lógicas
- Modo: sem intervalo (lotes 1 a 10000)
- Java (`java -version`): openjdk version "25.0.4.1" 2026-08-18 LTS / OpenJDK Runtime Environment Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS) / OpenJDK 64-Bit Server VM Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS, mixed mode, sharing)
- Spawn até java_ready: 73.9 ms; até primeiro eco: 76.9 ms
RESTART OK: 76.2 ms (kill, relançado, java_ready e eco)

## Sidecar (anel de memória)
| lote k | viagens | p50 µs | p99 µs | p99.9 µs | máx µs | msgs/s |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 20000 | 0.6 | 2.9 | 16.3 | 523.3 | 1666667 |
| 10 | 20000 | 1.1 | 4.5 | 17.0 | 169.9 | 9090909 |
| 100 | 20000 | 6.5 | 11.6 | 56.3 | 777.7 | 15384615 |
| 1000 | 20000 | 54.9 | 88.8 | 330.2 | 612.7 | 18214936 |
| 10000 | 2000 | 496.3 | 792.9 | 980.9 | 1005.0 | 20149103 |

## Linha de base (PipeEcho, pipes stdin/stdout)
| lote k | viagens | p50 µs | p99 µs | p99.9 µs | máx µs | msgs/s |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 20000 | 101.5 | 140.9 | 262.4 | 766.6 | 9852 |
| 10 | 20000 | 146.4 | 192.9 | 482.4 | 963.3 | 68306 |
| 100 | 20000 | 561.8 | 800.1 | 1189.9 | 1886.9 | 177999 |
| 1000 | 20000 | 4882.2 | 5833.1 | 8449.0 | 22012.6 | 204826 |

### gap0


- Plataforma: Windows, 16 threads lógicas
- Modo gap: intervalo de 0 µs entre viagens (fora da medição)
- Java (`java -version`): openjdk version "25.0.4.1" 2026-08-18 LTS / OpenJDK Runtime Environment Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS) / OpenJDK 64-Bit Server VM Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS, mixed mode, sharing)
- Spawn até java_ready: 64.9 ms; até primeiro eco: 67.7 ms
RESTART OK: 66.8 ms (kill, relançado, java_ready e eco)

## Sidecar (anel de memória)
| lote k | viagens | p50 µs | p99 µs | p99.9 µs | máx µs | msgs/s |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 20000 | 0.6 | 2.7 | 18.0 | 359.4 | 1666667 |
| 10 | 20000 | 1.1 | 4.3 | 9.7 | 134.5 | 9090909 |
| 100 | 20000 | 6.3 | 9.0 | 12.8 | 561.7 | 15873016 |
| 1000 | 20000 | 54.7 | 83.5 | 276.5 | 641.2 | 18281536 |

## Linha de base (PipeEcho, pipes stdin/stdout)
| lote k | viagens | p50 µs | p99 µs | p99.9 µs | máx µs | msgs/s |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 20000 | 88.0 | 130.9 | 213.5 | 482.3 | 11364 |
| 10 | 20000 | 138.5 | 185.3 | 271.9 | 906.8 | 72202 |
| 100 | 20000 | 565.2 | 738.3 | 1091.7 | 1489.2 | 176929 |
| 1000 | 20000 | 4938.4 | 5780.1 | 7224.3 | 50176.0 | 202495 |

### gap1000


- Plataforma: Windows, 16 threads lógicas
- Modo gap: intervalo de 1000 µs entre viagens (fora da medição)
- Java (`java -version`): openjdk version "25.0.4.1" 2026-08-18 LTS / OpenJDK Runtime Environment Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS) / OpenJDK 64-Bit Server VM Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS, mixed mode, sharing)
- Spawn até java_ready: 63.4 ms; até primeiro eco: 66.2 ms
RESTART OK: 76.2 ms (kill, relançado, java_ready e eco)

## Sidecar (anel de memória)
| lote k | viagens | p50 µs | p99 µs | p99.9 µs | máx µs | msgs/s |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 20000 | 28.8 | 215.5 | 376.0 | 896.1 | 34722 |
| 10 | 20000 | 32.9 | 216.8 | 410.1 | 829.2 | 303951 |
| 100 | 20000 | 43.8 | 79.2 | 306.3 | 5998.8 | 2283105 |
| 1000 | 20000 | 120.2 | 178.4 | 347.3 | 1303.4 | 8319468 |

## Linha de base (PipeEcho, pipes stdin/stdout)
| lote k | viagens | p50 µs | p99 µs | p99.9 µs | máx µs | msgs/s |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 20000 | 159.1 | 227.8 | 422.3 | 2021.2 | 6285 |
| 10 | 20000 | 215.4 | 298.8 | 476.0 | 2873.0 | 46425 |
| 100 | 20000 | 726.7 | 1126.2 | 1346.7 | 2931.5 | 137608 |
| 1000 | 20000 | 5277.5 | 7502.4 | 8140.6 | 10468.1 | 189484 |

### gap50000


- Plataforma: Windows, 16 threads lógicas
- Modo gap: intervalo de 50000 µs entre viagens (fora da medição)
- Java (`java -version`): openjdk version "25.0.4.1" 2026-08-18 LTS / OpenJDK Runtime Environment Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS) / OpenJDK 64-Bit Server VM Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS, mixed mode, sharing)
- Spawn até java_ready: 67.3 ms; até primeiro eco: 70.1 ms
RESTART OK: 76.3 ms (kill, relançado, java_ready e eco)

## Sidecar (anel de memória)
| lote k | viagens | p50 µs | p99 µs | p99.9 µs | máx µs | msgs/s |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 500 | 70.9 | 131.2 | 163.1 | 163.1 | 14104 |
| 10 | 500 | 64.8 | 114.5 | 311.9 | 311.9 | 154321 |
| 100 | 500 | 72.3 | 99.2 | 109.0 | 109.0 | 1383126 |
| 1000 | 500 | 162.6 | 219.6 | 832.0 | 832.0 | 6150062 |

## Linha de base (PipeEcho, pipes stdin/stdout)
| lote k | viagens | p50 µs | p99 µs | p99.9 µs | máx µs | msgs/s |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 500 | 247.4 | 338.9 | 539.5 | 539.5 | 4042 |
| 10 | 500 | 302.5 | 432.6 | 6348.9 | 6348.9 | 33058 |
| 100 | 500 | 825.5 | 2259.5 | 5681.0 | 5681.0 | 121139 |
| 1000 | 500 | 5239.6 | 8862.1 | 13542.8 | 13542.8 | 190854 |

---

## Passada 2

### default


- Plataforma: Windows, 16 threads lógicas
- Modo: sem intervalo (lotes 1 a 10000)
- Java (`java -version`): openjdk version "25.0.4.1" 2026-08-18 LTS / OpenJDK Runtime Environment Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS) / OpenJDK 64-Bit Server VM Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS, mixed mode, sharing)
- Spawn até java_ready: 69.6 ms; até primeiro eco: 72.7 ms
RESTART OK: 70.6 ms (kill, relançado, java_ready e eco)

## Sidecar (anel de memória)
| lote k | viagens | p50 µs | p99 µs | p99.9 µs | máx µs | msgs/s |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 20000 | 0.6 | 2.7 | 13.8 | 33.9 | 1666667 |
| 10 | 20000 | 1.2 | 4.0 | 9.8 | 173.6 | 8333333 |
| 100 | 20000 | 6.5 | 10.4 | 43.9 | 672.0 | 15384615 |
| 1000 | 20000 | 55.8 | 135.1 | 320.9 | 693.3 | 17921147 |
| 10000 | 2000 | 490.9 | 785.2 | 1058.5 | 1071.1 | 20370748 |

## Linha de base (PipeEcho, pipes stdin/stdout)
| lote k | viagens | p50 µs | p99 µs | p99.9 µs | máx µs | msgs/s |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 20000 | 96.1 | 175.4 | 371.1 | 1059.1 | 10406 |
| 10 | 20000 | 132.9 | 212.6 | 384.0 | 706.8 | 75245 |
| 100 | 20000 | 588.9 | 963.0 | 1284.5 | 1576.1 | 169808 |
| 1000 | 20000 | 5045.6 | 6979.4 | 8266.4 | 11234.2 | 198192 |

### gap0


- Plataforma: Windows, 16 threads lógicas
- Modo gap: intervalo de 0 µs entre viagens (fora da medição)
- Java (`java -version`): openjdk version "25.0.4.1" 2026-08-18 LTS / OpenJDK Runtime Environment Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS) / OpenJDK 64-Bit Server VM Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS, mixed mode, sharing)
- Spawn até java_ready: 67.9 ms; até primeiro eco: 70.7 ms
RESTART OK: 88.9 ms (kill, relançado, java_ready e eco)

## Sidecar (anel de memória)
| lote k | viagens | p50 µs | p99 µs | p99.9 µs | máx µs | msgs/s |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 20000 | 0.6 | 3.2 | 16.4 | 178.5 | 1666667 |
| 10 | 20000 | 0.9 | 4.4 | 13.1 | 164.9 | 11111111 |
| 100 | 20000 | 6.4 | 10.1 | 23.4 | 678.2 | 15625000 |
| 1000 | 20000 | 55.3 | 108.1 | 257.9 | 994.3 | 18083183 |

## Linha de base (PipeEcho, pipes stdin/stdout)
| lote k | viagens | p50 µs | p99 µs | p99.9 µs | máx µs | msgs/s |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 20000 | 87.5 | 139.0 | 207.7 | 766.5 | 11429 |
| 10 | 20000 | 137.8 | 214.0 | 432.5 | 1322.5 | 72569 |
| 100 | 20000 | 567.9 | 757.7 | 1135.1 | 1529.7 | 176087 |
| 1000 | 20000 | 4951.5 | 7981.6 | 15589.6 | 34698.9 | 201959 |

### gap1000


- Plataforma: Windows, 16 threads lógicas
- Modo gap: intervalo de 1000 µs entre viagens (fora da medição)
- Java (`java -version`): openjdk version "25.0.4.1" 2026-08-18 LTS / OpenJDK Runtime Environment Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS) / OpenJDK 64-Bit Server VM Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS, mixed mode, sharing)
- Spawn até java_ready: 75.3 ms; até primeiro eco: 78.6 ms
RESTART OK: 66.0 ms (kill, relançado, java_ready e eco)

## Sidecar (anel de memória)
| lote k | viagens | p50 µs | p99 µs | p99.9 µs | máx µs | msgs/s |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 20000 | 28.5 | 68.5 | 349.6 | 655.2 | 35088 |
| 10 | 20000 | 28.6 | 57.1 | 129.4 | 549.0 | 349650 |
| 100 | 20000 | 34.2 | 327.3 | 483.2 | 1344.2 | 2923977 |
| 1000 | 20000 | 86.2 | 397.9 | 767.1 | 1330.0 | 11600928 |

## Linha de base (PipeEcho, pipes stdin/stdout)
| lote k | viagens | p50 µs | p99 µs | p99.9 µs | máx µs | msgs/s |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 20000 | 125.2 | 190.4 | 451.2 | 1445.8 | 7987 |
| 10 | 20000 | 175.6 | 1805.0 | 17082.8 | 38255.8 | 56948 |
| 100 | 20000 | 662.1 | 1032.7 | 1217.8 | 1514.4 | 151035 |
| 1000 | 20000 | 5062.1 | 6315.6 | 8217.4 | 9170.3 | 197546 |

### gap50000


- Plataforma: Windows, 16 threads lógicas
- Modo gap: intervalo de 50000 µs entre viagens (fora da medição)
- Java (`java -version`): openjdk version "25.0.4.1" 2026-08-18 LTS / OpenJDK Runtime Environment Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS) / OpenJDK 64-Bit Server VM Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS, mixed mode, sharing)
- Spawn até java_ready: 63.5 ms; até primeiro eco: 66.1 ms
RESTART OK: 68.9 ms (kill, relançado, java_ready e eco)

## Sidecar (anel de memória)
| lote k | viagens | p50 µs | p99 µs | p99.9 µs | máx µs | msgs/s |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 500 | 50.3 | 298.9 | 399.8 | 399.8 | 19881 |
| 10 | 500 | 48.5 | 402.1 | 946.1 | 946.1 | 206186 |
| 100 | 500 | 51.7 | 313.7 | 499.0 | 499.0 | 1934236 |
| 1000 | 500 | 104.5 | 363.9 | 795.3 | 795.3 | 9569378 |

## Linha de base (PipeEcho, pipes stdin/stdout)
| lote k | viagens | p50 µs | p99 µs | p99.9 µs | máx µs | msgs/s |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 500 | 166.7 | 473.5 | 979.9 | 979.9 | 5999 |
| 10 | 500 | 236.7 | 474.8 | 586.3 | 586.3 | 42248 |
| 100 | 500 | 794.9 | 1267.5 | 1461.8 | 1461.8 | 125802 |
| 1000 | 500 | 5162.1 | 6465.1 | 7063.6 | 7063.6 | 193720 |
