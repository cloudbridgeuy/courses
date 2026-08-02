+++
title = "Preguntas puente"
+++

Estas preguntas cierran la sesión presencial y abren la remota. Conviene pensarlas mientras
lo visto hoy está fresco: se leyó el template y se actualizó el stack con un change set. No
buscar las respuestas de inmediato; intentar razonarlas desde lo que se hizo. Al comenzar la
sesión remota, se discuten antes de seguir con buenas prácticas y contenedores.

:::slide
## Preguntas puente

1. Cambiar `DesiredCount` a mano en la consola vs. con un change set: ¿qué es el
   *drift*?
2. Si una tarea se cae, ¿quién arranca otra y cómo sabe qué imagen usar?
3. Al borrar el stack, ¿sobreviven los datos de DynamoDB? ¿Por qué?
:::

---

## Pregunta 1

Se cambió el `DesiredCount` del servicio con un change set. ¿Qué pasaría si, en cambio,
se lo cambiara a mano desde la consola de ECS? ¿Qué pensaría CloudFormation del estado del
stack?

::: solucion
El cambio surtiría efecto. ECS pondría a correr el número de tareas indicado, pero el
**template y la realidad dejarían de coincidir**. Eso es *drift*. CloudFormation seguiría
creyendo que `DesiredCount` es el valor de la última actualización aplicada por el stack,
porque no se entera de los cambios hechos por fuera.

El problema aparece en la siguiente actualización del stack: CloudFormation podría
revertir el cambio manual sin avisar, al aplicar el valor del template. La herramienta
**Detect drift** existe justamente para detectar estas diferencias. La regla práctica:
si un recurso lo gestiona un stack, cámbielo solo a través del stack.
:::

---

## Pregunta 2

El servicio mantiene dos tareas en ejecución. Si una de ellas se cae, ¿quién arranca una
nueva, y cómo sabe esa tarea nueva qué imagen ejecutar?

::: solucion
Quien la arranca es el **servicio de ECS**. Un servicio no es solo "lanzar un
contenedor": es un controlador que vigila el número de tareas en ejecución contra el
`DesiredCount` deseado. Si una tarea termina o falla, el servicio nota la diferencia
(2 deseadas, 1 corriendo) y lanza una de reemplazo automáticamente.

La tarea nueva sabe qué imagen usar porque el servicio la crea a partir de su **task
definition**, que es la plantilla de ejecución: especifica la imagen, la CPU, la
memoria, los puertos. El servicio aporta el *cuántas y mantenerlas vivas*; la task
definition aporta el *cómo es cada una*. Esa distinción es el tema central de la próxima
sección.
:::

---

## Pregunta 3

Si se borra el stack de CloudFormation, ¿sobreviven los datos guardados en la tabla de
DynamoDB? ¿Por qué?

::: solucion
Con el template tal como está, **no**: la tabla la creó y la gestiona el stack, así que
al borrar el stack se borra la tabla y, con ella, los datos.

Esto se puede cambiar. CloudFormation permite declarar en un recurso una
`DeletionPolicy: Retain`, que le indica conservar ese recurso aunque se borre el stack.
Es una decisión deliberada: el ambiente de ejecución (clúster, servicio, balanceador)
tiene sentido destruirlo y recrearlo, pero los **datos** suelen querer sobrevivir a esos
ciclos. Veremos `DeletionPolicy` entre las buenas prácticas de la próxima sesión.
:::
