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

Las que importan para su aplicación:

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
CloudWatch Logs**, el mismo que identificó en la Semana 2. Ahí está el detalle de cada
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

1. Abra [**CloudWatch → Metrics → All metrics**](https://console.aws.amazon.com/cloudwatch/home).
2. Entre a **ECS → por servicio**, y seleccione `CPUUtilization` para su servicio.
3. Observe la gráfica. Ajuste el rango temporal (última hora, último día) en la esquina
   superior.

Para que la métrica tenga algo que mostrar, genere carga real en su pod con el botón de
abajo. El evento viaja a su propio servidor (mismo origen), así que la CPU se quema en su
task de ECS, y el pico aparece en *su* CloudWatch. El contador, en cambio, guarda estado
en DynamoDB y se lee de vuelta —un ejemplo del mismo contrato de eventos aplicado a datos
persistentes.

:::app
<cb-cpu-burst seconds="60" intensity="high" label="Generar carga de CPU (60 s)"></cb-cpu-burst>
<cb-counter key="demo" mode="increment" label="Incrementar contador"></cb-counter>
:::

### Consultar los logs

1. Abra **CloudWatch → Logs → Logs Insights**.
2. En el selector de grupos, elija el grupo de logs de su contenedor (el que vio en la
   task definition).
3. Pegue la consulta de las veinte líneas recientes y pulse **Run query**. Lea la salida
   de su aplicación.

Al pulsar los botones de arriba, las acciones dejan rastro en los logs. Busque líneas
como `cpu-burst started`, `counter incremented`, o `ignoring duplicate event id`: cada
una nombra el evento, el handler, y el resultado. Así se ve un log *útil* —cuenta qué
pasó, con qué datos, y cómo terminó— y eso es justo lo que se consulta con Logs Insights.

---

{#ejercicio-13}
### Ejercicio 13 — Lea la métrica y el log

Para su aplicación, abra la métrica `CPUUtilization` del servicio en CloudWatch, y
consulte las líneas de log más recientes del contenedor con Logs Insights.

::: solucion
1. Abra [**CloudWatch → Metrics → All metrics**](https://console.aws.amazon.com/cloudwatch/home).
2. Navegue a **ECS → por servicio** y seleccione `CPUUtilization` para su servicio.
   Observe la gráfica y ajuste el rango temporal.
3. Abra **CloudWatch → Logs → Logs Insights**.
4. Seleccione el grupo de logs de su contenedor (el de la task definition de la Semana
   2).
5. Ejecute la consulta:

   ```
   fields @timestamp, @message
   | sort @timestamp desc
   | limit 20
   ```

6. Lea las líneas devueltas: son la salida reciente de su aplicación en ejecución.
:::

:::slide light
{{ejercicio-13}}
:::

El contador `demo` incrementado arriba se refleja aquí en vivo, aunque esté en otra
parte de la página: ambos widgets comparten el mismo flujo de eventos por SSE.

:::app
<cb-counter key="demo" mode="view" label="Contador demo"></cb-counter>
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

Pasó de operar a mano a tener un sistema que se entrega y se reporta solo.

## Qué sigue en la Semana 4

La última semana cierra la observabilidad y el curso. Vamos a:

- Construir **dashboards** que reúnan las métricas clave en una sola vista, y
  **alarmas** que avisen —por el mismo camino a Teams— cuando un umbral se cruza.
- Activar **Container Insights** para ver el detalle por tarea y por servicio, e
  introducir la **trazabilidad operacional**: seguir un síntoma desde la métrica hasta la
  línea de log que lo explica.
- Cerrar con un **repaso del flujo completo** y un ejercicio integrador de extremo a
  extremo.

Llegará al final con el ciclo entero en la cabeza: del código a la imagen, al despliegue,
a la operación, y a la observación —y las herramientas para diagnosticar cuando algo se
sale de lo esperado.
