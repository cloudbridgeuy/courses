+++
title = "Dashboards y alarmas"
+++


## De leer métricas a vigilarlas

La Semana 3 abrió la observabilidad: ya se sabe leer una métrica y consultar un log. Pero
abrir la consola a buscar cada número no es vigilar un sistema. Esta semana cierra la
observabilidad con las dos herramientas que convierten métricas sueltas en vigilancia
real: los **dashboards**, que reúnen lo importante en una vista, y las **alarmas**, que
avisan sin que nadie tenga que mirar.

:::inline-slide
## Dashboards: una vista, varias métricas

:::skip
Un **dashboard** de CloudWatch es un tablero de gráficas que se compone. En lugar de
abrir cada métrica por separado, se juntan las que cuentan la historia del sistema en una
sola pantalla.
:::

Para la aplicación, un dashboard útil reúne:

- **Latencia** del ALB (`TargetResponseTime`) — cuán rápido responde.
- **Peticiones** (`RequestCount`) — cuánta carga llega.
- **Errores 5XX** (`HTTPCode_Target_5XX_Count`) — cuántas fallas devuelve.
- **CPU y memoria** del servicio ECS — cuántos recursos consume.
- **Destinos sanos** del target group — cuántas tareas reciben tráfico.

:::skip
Leídas juntas, estas gráficas responden de un vistazo: ¿el sistema está sano, y aguanta
la carga que recibe?
:::
:::

:::inline-slide
## Alarmas: que el sistema avise solo

Un dashboard sirve cuando alguien lo mira. Una **alarma** vigila una métrica todo el
tiempo y actúa cuando cruza un umbral, sin que nadie esté presente.

Una alarma tiene tres estados:

- **OK** — la métrica está dentro del umbral.
- **ALARM** — la métrica cruzó el umbral durante el período definido.
- **INSUFFICIENT_DATA** — no hay datos suficientes para evaluar (al inicio, o si la
  métrica deja de llegar).
:::

Lo valioso es la **acción** que dispara al entrar en ALARM: publicar en un **tema de
SNS**. Y ese es el mismo tema que ya alimenta las notificaciones del pipeline hacia
Teams. Así, una alarma de CPU alta o de errores 5XX llega al mismo canal donde el equipo
ya recibe los avisos del pipeline (sin montar nada nuevo.)

## Práctica guiada: dashboard y alarma
:::inline-slide light with-title

:::app
<cb-goto path="Práctica guiada: dashboard y alarma"></cb-goto>
:::
:::

### Crear el dashboard

1. Abrir [**CloudWatch → Dashboards**](https://console.aws.amazon.com/cloudwatch/home#dashboards:) y pulsar **Create dashboard**. Nombrarlo
   `taller-aws-<su-nombre>`.
2. Agregar un *widget* de línea con la métrica `CPUUtilization` del servicio de ECS.
3. Agregar otro widget con `TargetResponseTime` del ALB.
4. Agregar un tercero con `HTTPCode_Target_5XX_Count`. Guardar el dashboard.

::: info
Agentes de IA son excelentes para crear graficos complejos utilizando el format JSON de las gráficas.

```json
{
    "view": "timeSeries",
    "stacked": false,
    "title": "ALB - Ratio de Errores (4XX+5XX)",
    "stat": "Sum",
    "period": 300,
    "region": "us-east-2",
    "yAxis": {
        "left": {
            "min": 0,
            "label": "% de errores"
        }
    },
    "legend": {
        "position": "bottom"
    },
    "metrics": [
        [ { "expression": "100*(FILL(m1,0)+FILL(m4,0)+FILL(m7,0)+FILL(m8,0)+FILL(m9,0)+FILL(m10,0)+FILL(m11,0)+FILL(m12,0))/(FILL(m1,0)+FILL(m2,0)+FILL(m3,0)+FILL(m4,0)+FILL(m5,0)+FILL(m6,0)+FILL(m7,0)+FILL(m8,0)+FILL(m9,0)+FILL(m10,0)+FILL(m11,0)+FILL(m12,0))", "label": "Ratio de errores 4XX+5XX", "id": "e1", "region": "us-east-2" } ],
        [ "AWS/ApplicationELB", "HTTPCode_Target_4XX_Count", "TargetGroup", "targetgroup/taller-Grupo-UZQNL5TF2LT4/7ff4979298e3b4e2", "AvailabilityZone", "us-east-2b", "LoadBalancer", "app/taller-Balan-IATj255Q04JY/80c0fcb4397bea7a", { "id": "m1", "visible": false, "region": "us-east-2" } ],
        [ ".", "HTTPCode_Target_2XX_Count", ".", ".", ".", ".", ".", ".", { "id": "m2", "visible": false, "region": "us-east-2" } ],
        [ "...", "us-east-2c", ".", ".", { "id": "m3", "visible": false, "region": "us-east-2" } ],
        [ ".", "HTTPCode_Target_4XX_Count", ".", ".", ".", ".", ".", ".", { "id": "m4", "visible": false, "region": "us-east-2" } ],
        [ ".", "HTTPCode_Target_2XX_Count", ".", "targetgroup/taller-EcoGr-GRODKD0CYS03/3b54d29b20ceb938", ".", "us-east-2b", ".", ".", { "id": "m5", "visible": false, "region": "us-east-2" } ],
        [ "...", "targetgroup/taller-Grupo-DWHPBMV1GIM6/418f6146d34df78e", ".", ".", ".", "app/taller-Balan-sWHxWDGNv8XS/30054cd2a6ff4dc1", { "id": "m6", "visible": false, "region": "us-east-2" } ],
        [ ".", "HTTPCode_Target_4XX_Count", ".", ".", ".", ".", ".", ".", { "id": "m7", "visible": false, "region": "us-east-2" } ],
        [ "...", "targetgroup/taller-EcoGr-GRODKD0CYS03/3b54d29b20ceb938", ".", ".", ".", "app/taller-Balan-IATj255Q04JY/80c0fcb4397bea7a", { "id": "m8", "visible": false, "region": "us-east-2" } ],
        [ ".", "HTTPCode_Target_5XX_Count", ".", "targetgroup/taller-Grupo-UZQNL5TF2LT4/7ff4979298e3b4e2", ".", ".", ".", ".", { "id": "m9", "visible": false, "region": "us-east-2" } ],
        [ "...", "us-east-2c", ".", ".", { "id": "m10", "visible": false, "region": "us-east-2" } ],
        [ "...", "targetgroup/taller-EcoGr-GRODKD0CYS03/3b54d29b20ceb938", ".", "us-east-2b", ".", ".", { "id": "m11", "visible": false, "region": "us-east-2" } ],
        [ "...", "targetgroup/taller-Grupo-DWHPBMV1GIM6/418f6146d34df78e", ".", ".", ".", "app/taller-Balan-sWHxWDGNv8XS/30054cd2a6ff4dc1", { "id": "m12", "visible": false, "region": "us-east-2" } ]
    ]
}
```
:::

:::app
<cb-eco></cb-eco>
:::


### Crear la alarma de CPU

1. Abrir [**CloudWatch → Alarms → All alarms**](https://console.aws.amazon.com/cloudwatch/home#alarmsV2:) y pulsar **Create alarm**.
2. **Select metric**: `ECS → por servicio → CPUUtilization` del servicio.
3. En la condición, elegir **Greater than** con un umbral de `70` (por ciento), evaluado
   durante un período.
4. En **Notification**, seleccionar el **tema de SNS** del pod (el mismo
   `codestar-notifications-taller-aws-<su-nombre>` que recibe las notificaciones del
   pipeline).
5. Nombrarlo `cpu-alta-<su-nombre>` y crearlo.

La alarma comienza en `INSUFFICIENT_DATA`, pasa a `OK` cuando hay datos, y entraría en
`ALARM` —publicando en SNS— si la CPU superara el 70%.

---
