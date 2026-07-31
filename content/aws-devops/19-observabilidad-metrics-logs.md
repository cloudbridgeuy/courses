+++
title = "Observabilidad — métricas y logs"
+++

## Saber qué hace el sistema en ejecución

El sistema ya se construye, se despliega, y se notifica solo. Falta la última dimensión:
**saber qué hace mientras corre**. Cuando la aplicación responde lento, o devuelve
errores, o una tarea se reinicia, la respuesta no está en el código —está en lo que el
sistema emite mientras opera. Esa es la observabilidad, y en AWS empieza con dos fuentes:
**métricas** y **logs** de CloudWatch.

## Métricas: la salud en números

Una **métrica** es una serie de valores numéricos en el tiempo: uso de CPU, número de
peticiones, latencia, tareas en ejecución. AWS publica métricas automáticamente, sin
configuración, agrupadas por *namespace* (el servicio que las emite).

Las que importan para la aplicación:

| Namespace | Métrica | Qué dice |
| --- | --- | --- |
| `AWS/ECS` | `CPUUtilization` | Cuánta CPU usa el servicio (la que alimenta el auto scaling). |
| `AWS/ECS` | `MemoryUtilization` | Cuánta memoria usa el servicio. |
| `AWS/ApplicationELB` | `RequestCount` | Cuántas peticiones recibe el ALB. |
| `AWS/ApplicationELB` | `TargetResponseTime` | Cuánto tarda la aplicación en responder. |
| `AWS/ApplicationELB` | `HTTPCode_Target_5XX_Count` | Cuántos errores de servidor devuelven las tareas. |

Estas cinco, leídas juntas, cuentan una historia: cuánta carga llega, cuán rápido se
responde, cuántos errores hay, y cuántos recursos se consumen.

## Logs: el detalle de cada evento

Donde la métrica dice *cuánto*, el **log** dice *qué pasó*. Cada tarea de Fargate envía la
salida de su contenedor —todo lo que la aplicación escribe a la consola— a un **grupo de
CloudWatch Logs**, el mismo que se identificó en la Semana 2. Ahí está el detalle de cada
petición, cada error, cada arranque.

:::inline-slide light
## Métricas vs. logs

| Métricas | Logs |
| --- | --- |
| *Cuánto* | *Qué pasó* |
| Números en el tiempo | Líneas de texto con detalle |
| Tendencias y umbrales | Diagnóstico evento por evento |
:::

### Logs Insights

Buscar a mano en miles de líneas no escala. **CloudWatch Logs Insights** permite
consultar los logs con un lenguaje sencillo. Por ejemplo, las veinte líneas más recientes:

```
fields @timestamp, @message
| sort @timestamp desc
| limit 20
```

O contar errores en una ventana de tiempo filtrando por una palabra. La consulta se
ejecuta sobre el grupo de logs y devuelve resultados en segundos.

## Práctica guiada: leer métricas y logs

### Ver una métrica del servicio

1. Abrir [**CloudWatch → Metrics → All metrics**](https://console.aws.amazon.com/cloudwatch/home).
2. Entrar a **ECS → por servicio**, y seleccionar `CPUUtilization` para el servicio.
3. Observar la gráfica. Ajustar el rango temporal (última hora, último día) en la esquina
   superior.

Para que la métrica tenga algo que mostrar, se genera carga real en el pod con el botón de
abajo. El evento viaja al propio servidor (mismo origen), así que la CPU se quema en la
task de ECS, y el pico aparece en CloudWatch.

:::app
<cb-cpu-burst seconds="60" intensity="high" label="Generar carga de CPU (60 s)"></cb-cpu-burst>
:::

### Consultar los logs

1. Abrir [**CloudWatch → Logs → Logs Insights**](https://console.aws.amazon.com/cloudwatch/home#logsV2:logs-insights).
2. En el selector de grupos, elegir el grupo de logs del contenedor (el que se vio en la
   task definition).
3. Pegar la consulta de las veinte líneas recientes y pulsar **Run query**. Leer la salida
   de la aplicación.

Al pulsar los botones de arriba, las acciones dejan rastro en los logs. Buscar líneas
como `cpu-burst started`, `counter incremented`, o `ignoring duplicate event id`: cada
una nombra el evento, el handler, y el resultado. Así se ve un log *útil* —cuenta qué
pasó, con qué datos, y cómo terminó— y eso es justo lo que se consulta con Logs Insights.

### Métricas personalizadas

Las métricas que AWS publica solas describen la infraestructura: CPU, latencia,
peticiones. Una **métrica personalizada** la publica la propia aplicación, y mide lo
que el equipo define como salud del negocio —pedidos procesados, ítems en una cola,
reintentos— más allá de lo que el contenedor revela por fuera.

Hay dos vías para llevar un número a CloudWatch:

| Vía | Cómo llega | Permisos |
| --- | --- | --- |
| **Log / EMF** | La aplicación escribe una línea en formato EMF; CloudWatch extrae la métrica del grupo de logs. | Ninguno extra —usa el log que ya existe. |
| **API / PutMetricData** | La aplicación llama directamente a la API de CloudWatch. | Requiere `cloudwatch:PutMetricData` en el rol de la tarea. |

Enviar un valor por cada vía con los controles de abajo. Ambos publican en el namespace
`Taller/Custom`, métrica `CustomValue`, con la dimensión `method` (`emf` o `api`) que
distingue su origen. El valor se acota a 0–100 (límite del taller, no de CloudWatch).

:::app
<cb-metric mode="emf" label="Enviar métrica (log/EMF)"></cb-metric>
:::

:::app
<cb-metric mode="api" label="Enviar métrica (PutMetricData)"></cb-metric>
:::

Abrir [**CloudWatch → Metrics → Taller/Custom**](https://console.aws.amazon.com/cloudwatch/home#metricsV2:)
y observar `CustomValue` con sus dos valores de dimensión. El envío por EMF, además,
aparece como una línea de log —se la ve llegar en la Live Tail de la sección siguiente.

::: extra ¿Y esto qué tiene que ver con DevOps?
Las dos vías exigen colaboración entre quienes operan y quienes construyen la
aplicación; la diferencia está en si ese acuerdo queda implícito y frágil, o explícito
y versionado.

La vía de log ata a Ops con Dev de forma silenciosa y continua: el monitoreo depende
del formato de una línea que el equipo de desarrollo controla y puede cambiar en
cualquier `commit`. Una alarma sostenida sobre ese log puede callar sin que nadie lo
note. Aun así, la vía de log es, con diferencia, la más sencilla —no toca el código, no
pide dependencias ni permisos nuevos, porque la línea ya existe—. Esa simplicidad es
real, y muchas veces es la decisión correcta cuando el log es estable y el costo de una
alarma rota es bajo.

La vía de API invierte el balance: más ingeniería, permisos
(`cloudwatch:PutMetricData`) y, a veces, una dependencia más en la aplicación, a cambio
de un contrato explícito, versionado y resiliente al cambio. Como todo, se elige la
opción que mejor sirve a la situación —ninguna gana siempre—. Esa conversación, y no la
herramienta, es DevOps: obliga a la pregunta que define al equipo de desarrollo, ¿qué
significa, para quienes construyen la aplicación, que esté funcionando como corresponde?
:::

### Seguir los logs en vivo (Live Tail)

Logs Insights consulta el pasado. Cuando lo que importa es **ahora** —reproducir un
problema y verlo aparecer— sirve **CloudWatch Logs Live Tail**: una cola en vivo del
grupo de logs, línea a línea, a medida que la aplicación las emite.

1. Abrir [**CloudWatch → Logs → Live Tail**](https://console.aws.amazon.com/cloudwatch/home#logsV2:live-tail).
2. Seleccionar el grupo de logs del contenedor y pulsar **Start**.
3. Dejar la cola corriendo en una pestaña.

Para tener algo que ver, se fabrican líneas de log a demanda con el contador de abajo. Cada
incremento dispara un evento `counter` en el pod, que escribe en DynamoDB y emite la
línea `counter incremented` —que aparece en la cola casi al instante. El valor a la
derecha se actualiza en vivo por SSE, aunque viva en otra parte de la página: misma
acción, dos vistas del mismo flujo de eventos.

:::app
<cb-counter key="demo" mode="increment" label="Incrementar contador"></cb-counter>
<cb-counter key="demo" mode="view" label="Contador demo"></cb-counter>
:::

Pulsar el botón unas cuantas veces y observar las líneas `counter incremented` llegar en
orden a la Live Tail. Eso es tailing de logs: el mismo grupo que se consulta con Insights,
visto en tiempo real mientras se genera la actividad.

---

{#ejercicio-15}
### Ejercicio 15 — Leer la métrica y el log

Para la aplicación, abrir la métrica `CPUUtilization` del servicio en CloudWatch, y
consultar las líneas de log más recientes del contenedor con Logs Insights.

::: solucion
1. Abrir [**CloudWatch → Metrics → All metrics**](https://console.aws.amazon.com/cloudwatch/home).
2. Navegar a **ECS → por servicio** y seleccionar `CPUUtilization` para el servicio.
   Observar la gráfica y ajustar el rango temporal.
3. Abrir [**CloudWatch → Logs → Logs Insights**](https://console.aws.amazon.com/cloudwatch/home#logsV2:logs-insights).
4. Seleccionar el grupo de logs del contenedor (el de la task definition de la Semana
   2).
5. Ejecutar la consulta:

   ```
   fields @timestamp, @message
   | sort @timestamp desc
   | limit 20
   ```

6. Leer las líneas devueltas: son la salida reciente de la aplicación en ejecución.
:::

:::slide light
{{ejercicio-15}}
:::

---

## Dónde estamos

Al cerrar la Semana 3, el sistema no solo está en línea: está **operado, automatizado, y
observado**:

- **Opera** el workload: entiende el camino del tráfico, configuró el escalado
  automático, y sabe diagnosticar una tarea que no arranca.
- **Automatizó la entrega** con un pipeline de CodePipeline: del commit al despliegue,
  con disparo automático y una aprobación manual, y con notificaciones del pipeline hacia
  Teams (o, en el lab, hacia la guía).
- **Abrió la observabilidad**: lee las métricas de salud del servicio y consulta los logs
  del contenedor con Logs Insights.

Se pasó de operar a mano a tener un sistema que se entrega y se reporta solo.

## Qué sigue en la Semana 4

La última semana cierra la observabilidad y el curso. Se va a:

- Construir **dashboards** que reúnan las métricas clave en una sola vista, y
  **alarmas** que avisen —por el mismo camino a Teams— cuando un umbral se cruza.
- Activar **Container Insights** para ver el detalle por tarea y por servicio, e
  introducir la **trazabilidad operacional**: seguir un síntoma desde la métrica hasta la
  línea de log que lo explica.
- Cerrar con un **repaso del flujo completo** y un ejercicio integrador de extremo a
  extremo.

Se llegará al final con el ciclo entero en la cabeza: del código a la imagen, al despliegue,
a la operación, y a la observación —y las herramientas para diagnosticar cuando algo se
sale de lo esperado.
