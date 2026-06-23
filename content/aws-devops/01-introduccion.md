+++
title = "Introducción"
+++

:::inline-slide light
## ¿Qué es DevOps?

Una combinación de prácticas culturales, servicios y herramientas que aumentan la
capacidad de entregar software a alta velocidad.

El objetivo es unificar las áreas de desarrollo y operaciones, las cuales comúnmente
suelen estar completamente separadas, para conseguir un mejor flujo desde el código
a la aplicación desplegada.
:::

:::inline-slide light
## Pipeline de entrega continua

Busca automatizar el flujo desde los últimos cambios realizados a la aplicación
hasta su despliegue en producción, asegurando que cumple con los requisitos básicos
de calidad, así como con el contexto necesario para su monitoreo continuo en día 2.

```
Código → Build → Test → Deploy → Monitor
```
:::

:::inline-slide light
## Ejercicios de la sesión

1. Crear el repositorio, clonarlo desde su origen y subir el código a CodeCommit
2. Construir la imagen con CodeBuild
3. Publicar la imagen en ECR
4. Desplegar el template de CloudFormation para su despliegue inicial

Cada ejercicio incluye su solución oculta — botón **Ver solución** en la guía.
:::

:::slide
## Servicios utilizados durante el curso

{{tabla-servicios}}
:::

## La narrativa del taller

Durante cuatro semanas se desplegará y operará una aplicación web real de principio a fin,
sobre infraestructura de AWS. No se trata de ejercicios aislados: cada sesión avanza un paso
concreto de la misma historia. Al terminar la Semana 4, contaremos con un flujo completo
para el despliegue de una aplicación, desde su desarrollo hasta su operación:

**CodeCommit → CodeBuild → ECR → ECS → CloudWatch**

{#tabla-servicios}
| Servicio | Rol en el pipeline |
| --- | --- |
| CodeCommit | Repositorio de código fuente |
| CodeBuild | Construcción de la imagen |
| ECR | Registro de imágenes Docker |
| CloudFormation | Infraestructura como código |
| ECS + Fargate | Ejecución de contenedores |

:::slide light
## La narrativa del taller

Una sola historia a lo largo de cuatro semanas:

**CodeCommit → CodeBuild → ECR → ECS → CloudWatch**
:::

:::title-slide Semana 1
:::

## Semana 1

La primera semana establece los cimientos que el resto del taller supone conocidos. Al
terminar la sesión del viernes contaremos con:

- Un **repositorio de código** en CodeCommit, con el código de la aplicación ya cargado.
- Un **pipeline de integración continua** en CodeBuild que, cada vez que lo ejecuta,
  compila la imagen y la publica en Amazon ECR.
- La **aplicación en línea**: accesible desde el navegador a través de un Application
  Load Balancer, desplegada con un template de CloudFormation provisto por el instructor,
  sobre ECS/Fargate, conectada a una tabla de DynamoDB.

El template de CloudFormation se usa esta semana como una caja negra: se lanza y se
obtiene un ambiente funcional en minutos. Cómo está construida por dentro es el tema de
la Semana 2.

:::slide
## El seguro del taller

::: warning
En caso de que su sistema no funcione, utilizaremos CloudFormation para reiniciar
el estado de su `pod`, eliminando todos los recursos, para luego volverlos a crear.
:::
:::

## El mecanismo de recuperación

Desde el primer día se practica destruir el ambiente completo y recrearlo desde cero.
Esto no es un ejercicio de destrucción: es el seguro del taller. Si algo sale mal en
cualquier sesión posterior (una configuración equivocada, un recurso corrompido) se
borra el stack de CloudFormation, se espera a que termine, y se lo vuelve a lanzar con los mismos
parámetros, dejándolo en el punto de partida anterior. El costo de un error se reduce a
minutos de espera.

:::slide
## El mecanismo de recuperación

Desde el primer día practicamos **destruir y recrear** el ambiente completo.

Si algo sale mal: borre el stack de CloudFormation, espere a que termine, y vuelva a
lanzarlo con los mismos parámetros. El costo de un error se reduce a minutos de espera.
:::

## Cómo usar esta guía

Cada sección combina teoría breve con práctica guiada paso a paso. El esquema es
siempre el mismo:

1. **Teoría** (10–15 min): el instructor presenta el problema del día y el servicio de
   AWS que lo resuelve. El instructor utilizará diapositivas directamente conectadas a
   esta guía, por lo que puede seguir al instructor en cualquiera de las dos vistas.
2. **Práctica guiada**: siga los pasos numerados en la guía. Intente cada ejercicio por
   su cuenta antes de revelar la solución.
3. **Solución oculta**: si se atasca, pulse el botón **Ver solución** bajo cada ejercicio.
   Aparecerán los clics exactos. Al final de cada sesión todos los participantes quedan
   en el mismo punto.
4. **Preguntas puente**: al terminar la sesión del miércoles (presencial), aparecen dos
   o tres preguntas que conectan lo construido con lo que viene el viernes (remota).
   Piénselas antes de la siguiente sesión.

:::slide
## ¿Cómo utilizar esta guía?

1. **Teoría** — el problema del día y el servicio de AWS que lo resuelve.
2. **Práctica guiada** — siga los pasos numerados; intente antes de ver la solución.
3. **Solución oculta** — pulse **Ver solución** si se atasca.
4. **Preguntas puente** — conectan el miércoles (presencial) con el viernes (remota).
:::

:::slide
## Requisitos previos

- **Cuenta AWS** con permisos para CodeCommit, CodeBuild, ECR, CloudFormation,
  ECS/Fargate y DynamoDB.
- **Navegador** actualizado (Chrome o Firefox).
- **Template** `taller-semana1.yaml`, provisto por el instructor.
:::

## Requisitos previos

Antes de comenzar, confirme que cuenta con lo siguiente:

- **Cuenta AWS** con permisos para CodeCommit, CodeBuild, ECR, CloudFormation, ECS y
  Fargate, y DynamoDB. Si no está seguro, consulte con el instructor.
- **Navegador web** actualizado (Chrome o Firefox recomendados).
- **Template de CloudFormation** `taller-semana1.yaml`, también provisto por el
  instructor. La usará en la sección 4.
