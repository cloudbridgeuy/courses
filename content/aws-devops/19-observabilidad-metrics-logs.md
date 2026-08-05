+++
title = "Observabilidad — métricas y logs"
+++

## Saber qué hace el sistema en ejecución

El sistema ya se construye, se despliega, y se notifica solo. Falta la última dimensión:
**saber qué hace mientras corre**. Cuando la aplicación responde lento, o devuelve
errores, o una tarea se reinicia, la respuesta no está en el código. Está en lo que el
sistema emite mientras opera. Esa es la observabilidad, y en AWS empieza con dos fuentes:
**métricas** y **logs** de CloudWatch.

:::inline-slide
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
:::

## Logs: el detalle de cada evento

Donde la métrica dice *cuánto*, el **log** dice *qué pasó*. Cada tarea de Fargate envía la
salida de su contenedor —todo lo que la aplicación escribe a la consola— a un **grupo de
CloudWatch Logs**, el mismo que se identificó en la Semana 2. Ahí está el detalle de cada
petición, cada error, cada arranque.

## Métricas vs. logs
:::inline-slide light with-title

| Métricas | Logs |
| --- | --- |
| *Cuánto* | *Qué pasó* |
| Números en el tiempo | Líneas de texto con detalle |
| Tendencias y umbrales | Diagnóstico evento por evento |
:::

:::inline-slide light with-title
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
:::

:::inline-slide
## Práctica guiada: leer métricas y logs
:::add visibility=slide
:::app
<cb-goto path="Práctica guiada: leer métricas y logs"></cb-goto>
::: #app
::: #add
::: #inline-slide

### Ver una métrica del servicio

El menú **Metrics** de CloudWatch tiene cuatro entradas, y conviene saber cuál es cuál
antes de buscar:

| Entrada | Qué hace |
| --- | --- |
| **Query Studio** | La pantalla nueva: se **escribe** la consulta, en PromQL o en Metrics Insights, y la métrica se busca por nombre y por etiqueta, incluso entre varias cuentas y regiones. |
| **Classic metrics** | El árbol de siempre: se **navega** namespace → dimensión → métrica, y se marca lo que se quiere graficar. Es lo que usa el taller, porque enseña dónde vive cada métrica. |
| **Explorer** | Tableros automáticos que agrupan un mismo conjunto de métricas por etiqueta o por tipo de recurso. |
| **Streams** | Exportación continua de métricas hacia afuera de CloudWatch (S3, Firehose, un tercero). No sirve para mirar, sirve para sacar. |

Query Studio y Classic metrics leen exactamente los mismos datos. Cambia si se pide la
métrica escribiéndola, o si se llega a ella navegando.

1. Abrir [**CloudWatch → Metrics → Classic metrics**](https://console.aws.amazon.com/cloudwatch/home#metricsV2:).
2. Entrar a **ECS → por servicio**, y seleccionar `CPUUtilization` para el servicio.
3. Observar la gráfica. Ajustar el rango temporal (última hora, último día) en la esquina
   superior.

Para que la métrica tenga algo que mostrar, se genera carga real en el pod con el botón de
abajo. El evento viaja al propio servidor (mismo origen), así que la CPU se quema en la
task de ECS, y el pico aparece en CloudWatch.

:::app
<cb-cpu-burst seconds="60" intensity="high" label="Generar carga de CPU"></cb-cpu-burst>
:::

### Más detalle: activar Container Insights

`CPUUtilization` del servicio es un **promedio**. Con una sola tarea alcanza; con varias
esconde justo lo que interesa. Un promedio del 60% puede ser tres tareas trabajando
igual, o una al 100% y dos dormidas.

**Container Insights** es la capa que agrega ese detalle: recolecta métricas de CPU,
memoria, red, y disco con más granularidad que las básicas, las publica en el namespace
`ECS/ContainerInsights`, y arma vistas listas para usar. Viene apagada en la
configuración de la cuenta (**Account Settings**), y se decide **por clúster** —el ajuste
del clúster sobrescribe el de la cuenta.

Desde la consola:

1. Abrir [**ECS → Clusters**](https://console.aws.amazon.com/ecs/home) y seleccionar el clúster del taller.
2. Pulsar **Actions -> Update cluster**, y en **Monitoring** elegir el nivel de
   observabilidad. Son tres, y la diferencia está en hasta dónde baja el detalle:

| Opción | Qué recolecta | Cuándo sirve |
| --- | --- | --- |
| **Turned off** | Nada extra: solo las métricas que ECS ya publica en `AWS/ECS`, promediadas por clúster y por servicio. | Es el valor por defecto. |
| **Container Insights** | Métricas agregadas **por clúster y por servicio** en `ECS/ContainerInsights`, con logs de rendimiento consultables desde Logs Insights. | Ver la salud del servicio con más dimensiones que las básicas, al costo más bajo. |
| **Container Insights with enhanced observability** | Todo lo anterior, **más el detalle por tarea y por contenedor**, y navegación desde el agregado hasta la tarea concreta. | Aislar un problema: bajar del promedio a la tarea que se sale de lo normal. |

   Para el taller se elige **with enhanced observability**: sin el nivel de tarea, el
   promedio del servicio sigue escondiendo justo lo que se quiere ver.

3. Guardar. Las primeras métricas aparecen en unos minutos en
   [**CloudWatch → Insights → Container Insights**](https://console.aws.amazon.com/cloudwatch/home#container-insights:).

::: warning
El clúster lo crea CloudFormation, así que este cambio en la consola es *drift*: el
template y la realidad dejan de coincidir, y la próxima actualización del stack puede
revertirlo. La forma durable es declararlo en el template del stack de plataforma:

```yaml
ClusterCompartido:
  Type: AWS::ECS::Cluster
  Properties:
    ClusterName: !Ref AWS::StackName
    ClusterSettings:
      - Name: containerInsights
        Value: enhanced   # enabled = básico, disabled = apagado
```
:::

A diferencia de las métricas básicas, esta recolección tiene costo, y por eso se decide
clúster por clúster. En la Semana 4 se usa esta vista para bajar del promedio del
servicio a la tarea concreta que se sale de lo normal.

### Consultar los logs

En la consola, el menú **Logs** de CloudWatch tiene tres entradas:

| Entrada | Qué hay ahí |
| --- | --- |
| **Log Management** | El inventario: los grupos de logs y sus streams, con su retención, su clase, y su tamaño. Es *dónde vive* el log. |
| **Log Analytics** | La sala de consulta: reúne **Logs Insights**, **Live Tail**, y **Contributor Insights** en una sola pantalla, con varias consultas en pestañas. Es *cómo se lee* el log. |
| **Log Anomalies** | Lo que CloudWatch detecta solo: patrones que se salen del comportamiento histórico del grupo de logs. |

Consultar el pasado es Logs Insights, y Logs Insights vive dentro de Log Analytics. Una
consulta ahí se arma con tres piezas —**qué grupo**, **qué ventana de tiempo**, y **qué
comandos**—, y las tres viven en la misma pantalla:

1. Abrir [**CloudWatch → Logs → Log Analytics**](https://console.aws.amazon.com/cloudwatch/home#logsV2:).
2. **Grupo.** En la fila del selector, **Query by: All log groups**, escribir el nombre
   del grupo del contenedor en **Search log groups...** (el de la task definition), o
   pulsar **Browse** para elegirlo de la lista.

   La fila de arriba, con **Log group tags** y **Data sources**, es otra cosa: son
   **facetas**, filtros que acotan el universo de grupos antes de elegir. Como los grupos
   del taller los crea CloudFormation, llegan etiquetados solos, y en **Log group tags**
   aparece `aws:cloudformation:stack-name` con un valor por stack
   (`taller-aws-<su-nombre>-app` y `taller-aws-<su-nombre>-eco`). Filtrar por ahí es el
   atajo cuando la cuenta tiene cientos de grupos y no se recuerda el nombre exacto.
3. **Ventana.** Arriba a la derecha, fijar el rango: los atajos **5m**, **30m**,
   **1h**, **3h**, **12h**, o un rango propio. No es un adorno —marca cuánto log se
   escanea, y el escaneo es lo que se cobra—.
4. **Comandos.** El editor ya trae una primera línea que empieza con `SOURCE
   logGroups(...)`: es la traducción literal de los dos pasos anteriores, y se reescribe
   sola al cambiar el grupo o el rango. Debajo van los comandos. Pegar la consulta de las
   veinte líneas recientes, y pulsar **Run** (o `⌘+Enter`):

   ```
   fields @timestamp, @message
   | sort @timestamp desc
   | limit 20
   ```

5. Leer la salida de la aplicación en los resultados.

La misma pantalla trae el resto del instrumental: **Saved queries** y **Query History**
para no reescribir lo de siempre, **Discovered fields** con los campos que CloudWatch
reconoció solo en el log, **Analyze patterns** para agrupar las líneas que se repiten,
**Top N** para ver qué valores dominan, y una barra **Ask AI to write a query** que
redacta la consulta a partir de una descripción en lenguaje natural.

::: info
**Log Analytics** es la experiencia por defecto, y es reciente. Si la cuenta se dio de
baja de ella, las mismas tres herramientas aparecen como entradas sueltas del menú
(**Logs Insights**, **Live tail**, y **Contributor Insights**). El motor, el lenguaje de
consulta, y los precios son los mismos; cambia solo dónde se pulsa.
:::

Al pulsar los botones de arriba, las acciones dejan rastro en los logs. Buscar líneas
como `cpu-burst started`, `counter incremented`, o `ignoring duplicate event id`: cada
una nombra el evento, el handler, y el resultado. Así se ve un log *útil* —cuenta qué
pasó, con qué datos, y cómo terminó— y eso es justo lo que se consulta con Logs Insights.

### Seguir los logs en vivo (Live Tail)

Logs Insights consulta el pasado. Cuando lo que importa es **ahora** (reproducir un
problema y verlo aparecer) sirve **CloudWatch Logs Live Tail**: una cola en vivo del
grupo de logs, línea a línea, a medida que la aplicación las emite. No es otra pantalla:
es un botón de la misma Log Analytics.

1. Abrir [**CloudWatch → Logs → Log Analytics**](https://console.aws.amazon.com/cloudwatch/home#logsV2:) y pulsar **Live tail**, arriba a la derecha del editor.
2. Elegir el grupo de logs del contenedor, y arrancar la sesión. Opcionalmente, filtrar
   por un texto, o resaltar un término.
3. Dejar la cola corriendo en una pestaña del navegador.

::: warning
Una sesión de Live Tail se cobra **por minuto de sesión**, a diferencia de Logs Insights
que se cobra por volumen de log escaneado. Pulsar **Stop** al terminar la práctica.
:::

Para tener algo que ver, se fabrican líneas de log a demanda con el contador de abajo. Cada
incremento dispara un evento `counter` en el pod, que escribe en DynamoDB y emite la
línea `counter incremented`, que aparece en la cola casi al instante. El valor a la
derecha se actualiza en vivo por SSE, aunque viva en otra parte de la página: misma
acción, dos vistas del mismo flujo de eventos.

:::app
<cb-counter key="demo" mode="increment" label="Incrementar contador"></cb-counter>
<cb-counter key="demo" mode="view" label="Contador demo"></cb-counter>
:::

Pulsar el botón unas cuantas veces y observar las líneas `counter incremented` llegar en
orden a la Live Tail. Eso es tailing de logs: el mismo grupo que se consulta con Insights,
visto en tiempo real mientras se genera la actividad.

### Métricas personalizadas

Las métricas que AWS publica solas describen la infraestructura: CPU, latencia,
peticiones. Una **métrica personalizada** la publica la propia aplicación, y mide lo
que el equipo define como salud del negocio: pedidos procesados, ítems en una cola,
reintentos. Van más allá de lo que el contenedor revela por fuera.

Hay dos vías para llevar un número a CloudWatch:

| Vía | Cómo llega | Permisos |
| --- | --- | --- |
| **Log / EMF** | La aplicación escribe una línea en formato EMF; CloudWatch extrae la métrica del grupo de logs. | Ninguno extra, usa el log que ya existe. |
| **API / PutMetricData** | La aplicación llama directamente a la API de CloudWatch. | Requiere `cloudwatch:PutMetricData` en el rol de la tarea. |

Enviar un valor por cada vía con los controles de abajo. Ambos publican en el namespace
`Taller/Custom`, métrica `CustomValue`, con la dimensión `method` (`emf` o `api`) que
distingue su origen. El valor se acota a 0–100 (límite del taller, no de CloudWatch).

Cada control tiene dos botones. **Enviar** manda el número del campo, una sola vez.
**Auto (5 s)** arranca un envío repetido: un valor al azar cada cinco segundos, con la
cuenta regresiva del próximo a la derecha, hasta pulsar **Pausar**. La diferencia importa
al mirar la gráfica: un punto suelto no dibuja nada, y un período, un estadístico, o una
alarma necesitan una serie para tener de qué agarrarse. Arrancar el envío automático en
las dos vías ahora, y dejarlo corriendo mientras se busca la métrica en la consola.

:::app
<cb-metric mode="emf" label="Enviar métrica (log/EMF)"></cb-metric>
:::

:::app
<cb-metric mode="api" label="Enviar métrica (PutMetricData)"></cb-metric>
:::

Una métrica personalizada no aparece junto a las de ECS: vive en su propio namespace, y
hay que ir a buscarla.

1. Abrir [**CloudWatch → Metrics → Classic metrics**](https://console.aws.amazon.com/cloudwatch/home#metricsV2:).
2. En la pestaña **Browse**, mirar el bloque **Custom namespaces**, arriba del de
   **AWS namespaces**, y entrar a **Taller/Custom**. El namespace no existe hasta el
   primer envío: lo crea el primer dato que llega, así que si no está, pulsar el botón de
   arriba y esperar un minuto.
3. Entrar a la agrupación **method**: es la dimensión con la que la aplicación publica.
   Aparecen dos filas de `CustomValue`, una por vía (`emf` y `api`). Marcar las dos.
4. Ajustar la lectura en **Graphed metrics**: el estadístico en **Maximum**, el período
   en **1 minute**, y arriba el rango en **1h**. Con el envío automático corriendo, cada
   período de un minuto junta una docena de valores, y el estadístico decide qué se ve de
   ellos: **Maximum** el pico de la ventana, **Average** la media. Con envíos sueltos, en
   cambio, un período de cinco minutos promedia el único valor contra el vacío, y la línea
   casi desaparece.

Las dos series salen del mismo gesto, y llegan por caminos distintos. La de
`api` aparece casi de inmediato. La de `emf` tarda un poco más: primero se escribe la
línea de log, y después CloudWatch extrae el número de ahí. Esa misma línea se ve llegar
en la Live Tail de la sección anterior, si quedó una sesión abierta.

Al terminar, pulsar **Pausar** en los dos controles. El envío automático sigue mientras la
página esté abierta, y cada punto es una llamada a `PutMetricData`, o una línea de log
más: dejarlo corriendo toda la tarde se paga.

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

---

{#ejercicio-16}
### Ejercicio 16 — Leer la métrica y el log

Para la aplicación, abrir la métrica `CPUUtilization` del servicio en CloudWatch, y
consultar las líneas de log más recientes del contenedor con Logs Insights.

::: solucion
1. Abrir [**CloudWatch → Metrics → Classic metrics**](https://console.aws.amazon.com/cloudwatch/home#metricsV2:).
2. Navegar a **ECS → por servicio** y seleccionar `CPUUtilization` para el servicio.
   Observar la gráfica y ajustar el rango temporal.
3. Abrir [**CloudWatch → Logs → Log Analytics**](https://console.aws.amazon.com/cloudwatch/home#logsV2:), en la herramienta **Logs Insights**.
4. Seleccionar el grupo de logs del contenedor (el de la task definition de la Semana
   2) en **Search log groups...**, y fijar el rango de tiempo arriba a la derecha.
5. Escribir la consulta debajo de la línea `SOURCE`, y pulsar **Run**:

   ```
   fields @timestamp, @message
   | sort @timestamp desc
   | limit 20
   ```

6. Leer las líneas devueltas: son la salida reciente de la aplicación en ejecución.
:::

:::slide light
{{ejercicio-16}}
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
- Explotar **Container Insights** para ver el detalle por tarea y por servicio, e
  introducir la **trazabilidad operacional**: seguir un síntoma desde la métrica hasta la
  línea de log que lo explica.
- Cerrar con un **repaso del flujo completo** y un ejercicio integrador de extremo a
  extremo.

Se llegará al final con el ciclo entero en la cabeza: del código a la imagen, al despliegue,
a la operación, y a la observación —y las herramientas para diagnosticar cuando algo se
sale de lo esperado.
