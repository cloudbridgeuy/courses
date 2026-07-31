+++
title = "Dashboards y alarmas"
+++

:::title-slide Semana 4
:::

## De leer métricas a vigilarlas

La Semana 3 abrió la observabilidad: ya se sabe leer una métrica y consultar un log. Pero
abrir la consola a buscar cada número no es vigilar un sistema. Esta semana cierra la
observabilidad con las dos herramientas que convierten métricas sueltas en vigilancia
real: los **dashboards**, que reúnen lo importante en una vista, y las **alarmas**, que
avisan sin que nadie tenga que mirar.

## Dashboards: una vista, varias métricas

Un **dashboard** de CloudWatch es un tablero de gráficas que se compone. En lugar de
abrir cada métrica por separado, se juntan las que cuentan la historia del sistema en una
sola pantalla. Para la aplicación, un dashboard útil reúne:

- **Latencia** del ALB (`TargetResponseTime`) — cuán rápido responde.
- **Peticiones** (`RequestCount`) — cuánta carga llega.
- **Errores 5XX** (`HTTPCode_Target_5XX_Count`) — cuántas fallas devuelve.
- **CPU y memoria** del servicio ECS — cuántos recursos consume.
- **Destinos sanos** del target group — cuántas tareas reciben tráfico.

Leídas juntas, estas gráficas responden de un vistazo: ¿el sistema está sano, y aguanta
la carga que recibe?

:::inline-slide light
## Un dashboard que cuenta la historia

- **Latencia** — ¿responde rápido?
- **Peticiones** — ¿cuánta carga llega?
- **Errores 5XX** — ¿cuántas fallas?
- **CPU / memoria** — ¿cuántos recursos consume?
- **Destinos sanos** — ¿cuántas tareas sirven tráfico?
:::

## Alarmas: que el sistema avise solo

Un dashboard sirve cuando alguien lo mira. Una **alarma** vigila una métrica todo el
tiempo y actúa cuando cruza un umbral, sin que nadie esté presente.

Una alarma tiene tres estados:

- **OK** — la métrica está dentro del umbral.
- **ALARM** — la métrica cruzó el umbral durante el período definido.
- **INSUFFICIENT_DATA** — no hay datos suficientes para evaluar (al inicio, o si la
  métrica deja de llegar).

Lo valioso es la **acción** que dispara al entrar en ALARM: publicar en un **tema de
SNS**. Y ese es el mismo tema que ya alimenta las notificaciones del pipeline hacia
Teams. Así, una alarma de CPU alta o de errores 5XX llega al mismo canal donde el equipo
ya recibe los avisos del pipeline —sin montar nada nuevo.

:::slide
## Una alarma

```
Métrica cruza el umbral
  → estado ALARM
  → acción: publicar en SNS
  → (mismo tema) → Teams / toast
```

Vigila sola; avisa por el canal que ya usa el equipo.
:::

## Práctica guiada: dashboard y alarma

### Crear el dashboard

1. Abrir [**CloudWatch → Dashboards**](https://console.aws.amazon.com/cloudwatch/home) y pulsar **Create dashboard**. Nombrarlo
   `taller-<su-nombre>`.
2. Agregar un *widget* de línea con la métrica `CPUUtilization` del servicio de ECS.
3. Agregar otro widget con `TargetResponseTime` del ALB.
4. Agregar un tercero con `HTTPCode_Target_5XX_Count`. Guardar el dashboard.

### Crear la alarma de CPU

1. Abrir [**CloudWatch → Alarms → All alarms**](https://console.aws.amazon.com/cloudwatch/home#alarmsV2:) y pulsar **Create alarm**.
2. **Select metric**: `ECS → por servicio → CPUUtilization` del servicio.
3. En la condición, elegir **Greater than** con un umbral de `70` (por ciento), evaluado
   durante un período.
4. En **Notification**, seleccionar el **tema de SNS** del taller (el mismo de las
   notificaciones del pipeline).
5. Nombrarlo `cpu-alta-<su-nombre>` y crearlo.

La alarma comienza en `INSUFFICIENT_DATA`, pasa a `OK` cuando hay datos, y entraría en
`ALARM` —publicando en SNS— si la CPU superara el 70%.

---

{#ejercicio-16}
### Ejercicio 16 — Componer un dashboard y armar una alarma

Crear un dashboard de CloudWatch con, al menos, la CPU del servicio, la latencia del ALB,
y los errores 5XX. Luego crear una alarma sobre la CPU del servicio (umbral 70%) cuya
acción publique en el tema de SNS del taller.

::: solucion
1. Abrir [**CloudWatch → Dashboards**](https://console.aws.amazon.com/cloudwatch/home), pulsar **Create dashboard** y nombrarlo
   `taller-<su-nombre>`.
2. Agregar widgets de línea para `CPUUtilization` (ECS, el servicio),
   `TargetResponseTime` y `HTTPCode_Target_5XX_Count` (ALB). Guardar.
3. Abrir [**CloudWatch → Alarms → Create alarm**](https://console.aws.amazon.com/cloudwatch/home#alarmsV2:).
4. **Select metric**: `CPUUtilization` del servicio de ECS.
5. Condición: **Greater than**, umbral **70**.
6. **Notification**: el tema de SNS del taller.
7. Nombrarlo `cpu-alta-<su-nombre>` y crearlo. Confirmar que arranca en
   `INSUFFICIENT_DATA` y luego pasa a `OK`.
:::

:::slide light
{{ejercicio-16}}
:::
