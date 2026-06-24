+++
title = "Container Insights y trazabilidad"
+++

## Más detalle, y cómo conectarlo

Las métricas básicas dicen cómo está el servicio en conjunto. Pero cuando algo falla, la
pregunta suele ser más fina: ¿qué tarea consume de más?, ¿desde cuándo?, ¿qué decía su
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

## Práctica guiada: activar y recorrer

### Activar Container Insights

1. Abra [**ECS → Clusters**](https://console.aws.amazon.com/ecs/home) y seleccione su clúster.
2. En **Update cluster**, active **Container Insights**. Guarde.
3. Tras unos minutos, abra [**CloudWatch → Insights → Container Insights**](https://console.aws.amazon.com/cloudwatch/home#container-insights:) y seleccione su
   clúster: verá las métricas por servicio y por tarea.

### Recorrer un hilo

1. En Container Insights, observe la CPU **por tarea** de su servicio.
2. Anote la ventana de tiempo de cualquier pico (o del período reciente).
3. Abra [**Logs Insights**](https://console.aws.amazon.com/cloudwatch/home#logsV2:logs-insights), seleccione el grupo de logs del contenedor, y consulte las
   líneas de esa ventana:

   ```
   fields @timestamp, @message
   | sort @timestamp desc
   | limit 50
   ```

4. Lea qué hacía la aplicación en ese intervalo. Acaba de recorrer el hilo de la métrica
   al log.

---

{#ejercicio-15}
### Ejercicio 15 — Active Insights y siga un hilo

Active Container Insights en su clúster. Luego recorra el camino completo: observe la CPU
por tarea de su servicio, identifique una ventana de tiempo, y lea en Logs Insights las
líneas del contenedor en esa ventana.

::: solucion
1. Abra [**ECS → Clusters**](https://console.aws.amazon.com/ecs/home), seleccione su clúster, y en **Update cluster**
   active **Container Insights**. Guarde.
2. Espere unos minutos; abra [**CloudWatch → Insights → Container Insights**](https://console.aws.amazon.com/cloudwatch/home#container-insights:) y seleccione
   su clúster.
3. Observe la métrica de **CPU por tarea** de su servicio y anote una ventana de tiempo
   reciente.
4. Abra [**CloudWatch → Logs → Logs Insights**](https://console.aws.amazon.com/cloudwatch/home#logsV2:logs-insights), seleccione el grupo de logs del
   contenedor, y ejecute:

   ```
   fields @timestamp, @message
   | sort @timestamp desc
   | limit 50
   ```

5. Lea las líneas del intervalo: recorrió el camino de la métrica (qué tarea, cuándo) al
   log (qué hacía). Ese es el método de diagnóstico operacional.
:::

:::slide light
{{ejercicio-15}}
:::
