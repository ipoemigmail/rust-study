import $ivy.`dev.zio::zio:1.0.0-RC18-2`

import zio._

object Main {
  def main() {
    //val rt = new DefaultRuntime {}
    //val rt = Runtime(ZEnv.live, zio.internal.Platform.default)
    val rt = Runtime.unsafeFromLayer(ZEnv.live)
    val r = ZIO.collectAll((0 to 1000000).map(n => ZIO(n).fork.flatMap(_.join)))
    rt.unsafeRun(r)

    println("start")
    val startTime = System.currentTimeMillis
    rt.unsafeRun(r)
    val endTime = System.currentTimeMillis
    println(s"spend time: ${endTime - startTime}")
  }
}

Main.main()
