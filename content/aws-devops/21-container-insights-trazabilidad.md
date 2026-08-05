+++
title = "Container Insights y trazabilidad"
+++

## Más detalle, y cómo conectarlo

Las métricas básicas dicen cómo está el servicio en conjunto. Pero cuando algo falla, la
pregunta suele ser más fina: ¿qué tarea consume de más?, ¿desde cuándo?, ¿qué decía el
log en ese momento? Esta sección agrega el detalle (**Container Insights**) y la
disciplina para conectarlo (**trazabilidad**): seguir un síntoma desde la métrica hasta
la línea de log que lo explica.

## Container Insights

**Container Insights** es una capa de observabilidad específica para contenedores.
Activada sobre el clúster, recolecta métricas más detalladas que las básicas: uso de CPU
y memoria **por tarea** y **por servicio**, número de tareas en ejecución, y vistas de
rendimiento listas para usar.

La diferencia con las métricas de la Semana 3: aquellas dan el promedio del servicio;
Container Insights permite ver **cuál tarea** se sale de lo normal, no solo que el
promedio subió. Cuando un servicio tiene varias tareas y una se comporta distinto, ese
detalle es la diferencia entre adivinar y diagnosticar.

:::inline-slide light
## Dos niveles de detalle

| Métricas básicas | Container Insights |
| --- | --- |
| Promedio del servicio | Detalle por tarea |
| "La CPU subió" | "Esta tarea es la que subió" |
| Sin costo adicional | Recolección adicional (tiene costo) |
:::

::: extra Container Insights tiene un costo
A diferencia de las métricas básicas, Container Insights recolecta y almacena datos
adicionales, y eso tiene un costo asociado en CloudWatch. En un ambiente real se activa
donde el detalle justifica el gasto. Para el taller lo activamos para verlo funcionar; en
producción es una decisión por clúster.
:::

## Trazabilidad: de un síntoma a una causa

La observabilidad no es acumular gráficas: es poder **seguir un hilo**. Un usuario
reporta que la aplicación va lenta. ¿Por dónde se empieza? La trazabilidad es el camino
ordenado de un síntoma visible hasta la causa concreta.

:::slide
## De un síntoma a la causa

```
Síntoma (usuario: "va lento")
  → métrica del ALB (TargetResponseTime ↑)
  → métrica de ECS (¿qué tarea? CPU ↑)
  → log del contenedor (¿qué hacía esa tarea?)
```

Cada paso acota; el log da la respuesta.
:::

El recorrido típico:

1. **El síntoma**: el usuario percibe lentitud, o el dashboard muestra latencia alta
   (`TargetResponseTime`).
2. **Acotar en el ALB**: ¿es toda la aplicación o algunas peticiones? ¿coincide con un
   pico de `RequestCount` o de errores 5XX?
3. **Bajar a ECS / Container Insights**: ¿hay una tarea con CPU o memoria al límite?
   ¿cuándo empezó?
4. **Leer el log**: en el grupo de logs de esa tarea, en esa ventana de tiempo, buscar
   qué estaba haciendo —con Logs Insights, filtrando por el período del síntoma.

Cada paso reduce el espacio de búsqueda; el log es donde casi siempre está la respuesta
final. Esa secuencia —métrica que alerta, métrica que acota, log que explica— es el
método de troubleshooting operacional que el taller deja como herramienta.

### El hilo tiene nombre: `X-Amzn-Trace-Id`

Ese recorrido une métricas y logs por **tiempo**: se acota una ventana, y se lee lo que
pasó adentro. Funciona, y es aproximado —en un minuto de tráfico real caben miles de
peticiones, y solo una es la del síntoma—.

Hay una forma más fina, y ya está puesta desde el primer día. El ALB agrega a cada
petición que reenvía una cabecera **`X-Amzn-Trace-Id`**, con un identificador único:

```
X-Amzn-Trace-Id: Root=1-63f4a2b1-3c8e9d7a5b2f1e0c4d6a8b9e
```

No hace falta instrumentar nada para verla: el servidor de eco de la Semana 2 devuelve
todas las cabeceras que recibe, incluida esta.

```bash
curl -s "<UrlBase>/eco/" | grep -i trace
```

Ese valor es el hilo. Si la aplicación lo lee y lo escribe en cada línea de log, las
ventanas de tiempo dejan de hacer falta: una consulta por el identificador devuelve todo
lo que esa petición —y solo esa— produjo, aunque haya cruzado varias tareas.

```
fields @timestamp, @message
| filter @message like /1-63f4a2b1-3c8e9d7a5b2f1e0c4d6a8b9e/
| sort @timestamp asc
```

El mismo valor aparece en el campo `trace_id` del log de acceso del ALB, si se activa. Y
el prefijo `Root=` no es casual: es el formato de traza de **AWS X-Ray**, así que ese
identificador es el que uniría los segmentos del recorrido el día que se active.

Correlacionar por identificador, en vez de por reloj, es el paso que separa la
observabilidad de mirar gráficas.

## Práctica guiada: activar y recorrer

### Activar Container Insights

1. Abrir [**ECS → Clusters**](https://console.aws.amazon.com/ecs/home) y seleccionar el clúster.
2. En **Update cluster**, activar **Container Insights**. Guardar. Si ya quedó activado
   en la Semana 3, saltar al paso siguiente.
3. Tras unos minutos, abrir [**CloudWatch → Insights → Container Insights**](https://console.aws.amazon.com/cloudwatch/home#container-insights:) y seleccionar el
   clúster: se verán las métricas por servicio y por tarea.

### Recorrer un hilo

1. En Container Insights, observar la CPU **por tarea** del servicio de la aplicación
   —la vista lista los dos servicios del clúster por separado—.
2. Anotar la ventana de tiempo de cualquier pico (o del período reciente).
3. Abrir [**CloudWatch → Logs → Log Analytics**](https://console.aws.amazon.com/cloudwatch/home#logsV2:), en **Logs Insights**; seleccionar el grupo de logs
   del contenedor, y consultar las líneas de esa ventana:

   ```
   fields @timestamp, @message
   | sort @timestamp desc
   | limit 50
   ```

4. Leer qué hacía la aplicación en ese intervalo. Con esto se recorre el hilo de la métrica
   al log.

---

{#ejercicio-18}
### Ejercicio 18 — Activar Insights y seguir un hilo

Activar Container Insights en el clúster. Luego recorrer el camino completo: observar la CPU
por tarea del servicio, identificar una ventana de tiempo, y leer en Logs Insights las
líneas del contenedor en esa ventana.

::: solucion
1. Abrir [**ECS → Clusters**](https://console.aws.amazon.com/ecs/home), seleccionar el clúster, y en **Update cluster**
   activar **Container Insights**. Guardar.
2. Esperar unos minutos; abrir [**CloudWatch → Insights → Container Insights**](https://console.aws.amazon.com/cloudwatch/home#container-insights:) y seleccionar
   el clúster.
3. Observar la métrica de **CPU por tarea** del servicio y anotar una ventana de tiempo
   reciente.
4. Abrir [**CloudWatch → Logs → Log Analytics**](https://console.aws.amazon.com/cloudwatch/home#logsV2:), en **Logs Insights**; seleccionar el grupo de logs
   del contenedor, y ejecutar:

   ```
   fields @timestamp, @message
   | sort @timestamp desc
   | limit 50
   ```

5. Leer las líneas del intervalo: se recorrió el camino de la métrica (qué tarea, cuándo) al
   log (qué hacía). Ese es el método de diagnóstico operacional.
:::

:::slide light
{{ejercicio-18}}
:::
