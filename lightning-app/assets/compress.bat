for /r %%i in (*.png) do compressonatorcli -fd BC7 %%i %%~ni.dds"
for /r %%i in (*.jpg) do compressonatorcli -fd BC7 %%i %%~ni.dds"
